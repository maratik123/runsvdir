use anyhow::Error;
use base64ct::{Base64Unpadded, Encoding};
use nix::NixPath;
use sha2::{Digest, Sha256};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Shash {
    hash: [u8; 32],
    path: PathBuf,
}

impl PartialEq<Self> for Shash {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for Shash {}

impl Hash for Shash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl Display for Shash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} [{:?}]",
            Base64Unpadded::encode_string(&self.hash),
            self.path
        )
    }
}

impl TryFrom<&Path> for Shash {
    type Error = Error;

    fn try_from(path: &Path) -> anyhow::Result<Self> {
        let mut hasher = Sha256::new();
        hasher.update(path.as_os_str().as_encoded_bytes());
        hasher.update([0u8]);
        hasher.update(path.len().to_ne_bytes());
        hasher.update([0u8]);
        let mut buffer = [0; 1024];
        let mut file = BufReader::new(File::open(path)?);
        let mut total_len = 0usize;
        loop {
            match file.read(&mut buffer) {
                Ok(0) => {
                    hasher.update([0u8]);
                    hasher.update(total_len.to_ne_bytes());
                    break Ok(Self {
                        path: PathBuf::from(path),
                        hash: hasher.finalize().into(),
                    });
                }
                Ok(len) => {
                    hasher.update(&buffer[..len]);
                    total_len += len;
                }
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) => break Err(err.into()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn shash_test() {
        let path = PathBuf::from("test_res/c/run");
        assert_eq!(
            Shash::try_from(path.as_path()).unwrap(),
            Shash {
                hash: [
                    140, 13, 199, 163, 113, 234, 108, 171, 158, 185, 232, 3, 23, 64, 23, 168, 47,
                    225, 166, 225, 9, 144, 22, 10, 139, 30, 247, 7, 113, 208, 142, 45
                ],
                path
            }
        );
    }

    #[test]
    fn shash_non_existent_test() {
        let path = PathBuf::from("test_res/non_existent");
        assert_eq!(
            &Shash::try_from(path.as_path())
                .unwrap_err()
                .chain()
                .map(|err| err.downcast_ref::<io::Error>().map(|err| err.kind()))
                .collect::<Vec<_>>(),
            &[Some(ErrorKind::NotFound)]
        );
    }

    #[test]
    fn shash_display_test() {
        let path = PathBuf::from("test_res/c/run");
        assert_eq!(
            &Shash::try_from(path.as_path()).unwrap().to_string(),
            "jA3Ho3HqbKueuegDF0AXqC/hpuEJkBYKix73B3HQji0 [\"test_res/c/run\"]"
        );
    }
}