use clap::Parser;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use runsvdir::Shash;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fs::read_dir;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use std::{io, thread};
use tracing::{error, info};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// The number of millis to wait between each check
    #[clap(short, long, default_value = "1000")]
    pause: u64,
    /// The directory to store process states
    dir: PathBuf,
}

fn step(dir: &Path, running: &mut HashMap<Shash, Child>) {
    let mut cur = HashSet::new();

    for d in match read_dir(dir) {
        Ok(d) => d,
        Err(err) => {
            error!("read {dir:?} failed: {err}");
            return;
        }
    } {
        let d = match d {
            Ok(d) => d,
            Err(err) => {
                error!("skip dir entry due to {err}");
                continue;
            }
        };
        let mut p = d.path();
        p.push("run");

        let hash: Shash = match p.as_path().try_into() {
            Ok(hash) => hash,
            Err(err) => {
                error!("skip {p:?} due to {err}");
                continue;
            }
        };
        if let Entry::Vacant(e) = running.entry(hash.clone()) {
            info!("spawn {hash}");
            e.insert(
                match Command::new(p.as_os_str())
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    Ok(child) => child,
                    Err(err) => {
                        error!("spawn {hash} failed: {err}");
                        continue;
                    }
                },
            );
        } else {
            info!("{hash} is already running");
        }
        cur.insert(hash);
    }

    running.retain(|hash, child| {
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
}

fn main() {
    let args = Args::parse();

    FmtSubscriber::builder()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with_writer(io::stderr)
        .init();

    let mut running = HashMap::new();
    let pause = Duration::from_millis(args.pause);

    loop {
        step(&args.dir, &mut running);
        thread::sleep(pause);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_test() {
        let mut running = HashMap::new();
        step(Path::new("test_res"), &mut running);

        assert_eq!(
            running.keys().collect::<HashSet<_>>(),
            HashSet::from([
                &Shash::try_from(Path::new("test_res/b/run")).unwrap(),
                &Shash::try_from(Path::new("test_res/d/run")).unwrap()
            ])
        );

        for child in running.values_mut() {
            let _ = child.kill();
        }
    }
}
