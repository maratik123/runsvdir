use crate::Shash;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fs::read_dir;
use std::io;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use thiserror::Error;
use tracing::{error, info};

pub struct Stepper {
    dir: PathBuf,
    running: HashMap<Shash, Child>,
}

impl Stepper {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            running: HashMap::new(),
        }
    }

    pub fn invoke(&mut self) -> Result<(), StepError> {
        let mut cur = HashSet::new();

        for d in read_dir(self.dir.as_path())
            .map_err(|err| StepError::ReadDir(self.dir.as_path().into(), err))?
        {
            let f = || {
                let d = d.map_err(|err| StepError::ReadDirEntry(self.dir.as_path().into(), err))?;
                let mut p = d.path();
                p.push("run");

                let hash: Shash = p
                    .as_path()
                    .try_into()
                    .map_err(|err| StepError::Shash(p.clone(), err))?;
                if let Entry::Vacant(e) = self.running.entry(hash.clone()) {
                    info!("spawn {hash}");
                    e.insert(
                        Command::new(p.as_os_str())
                            .stdin(Stdio::null())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn()
                            .map_err(|err| StepError::Spawn(hash.clone(), err))?,
                    );
                } else {
                    info!("{hash} is already running");
                }
                cur.insert(hash);
                Ok::<_, StepError>(())
            };

            if let Err(err) = f() {
                error!("skipping entry, err: {err}");
            }
        }

        self.running.retain(|hash, child| {
            if !cur.contains(hash) {
                info!("{hash} stale");
                if let Err(err) = kill(Pid::from_raw(child.id() as i32), Signal::SIGTERM) {
                    error!("kill {hash} failed: {err}");
                }
            }
            match child.try_wait() {
                Ok(None) => {
                    info!("{hash} alive");
                    true
                }
                Ok(Some(status)) => {
                    info!("{hash} dead with {status}");
                    false
                }
                Err(err) => {
                    error!("get exit status for {hash} failed: {err}");
                    true
                }
            }
        });
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum StepError {
    #[error("Reading dir {0:?} failed: {1}")]
    ReadDir(PathBuf, #[source] io::Error),
    #[error("Reading dir entry on {0:?} failed: {1}")]
    ReadDirEntry(PathBuf, #[source] io::Error),
    #[error("Hashing on {0:?} failed: {1}")]
    Shash(PathBuf, #[source] io::Error),
    #[error("Spawn process {0} failed: {1}")]
    Spawn(Shash, #[source] io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn step_test() {
        let mut stepper = Stepper::new(PathBuf::from("test_res"));
        stepper.invoke().unwrap();

        assert_eq!(
            stepper.running.keys().collect::<HashSet<_>>(),
            HashSet::from([
                &Shash::try_from(Path::new("test_res/b/run")).unwrap(),
                &Shash::try_from(Path::new("test_res/d/run")).unwrap()
            ])
        );

        for child in stepper.running.values_mut() {
            let _ = child.kill();
        }
    }
}
