use base64ct::{Base64Unpadded, Encoding};
use nix::NixPath;
use sha2::{Digest, Sha256};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::hash::Hash;
use std::io;
use std::io::{BufReader, ErrorKind};
use std::path::{Path, PathBuf};
use uninit::extension_traits::AsOut;
use uninit::read::ReadIntoUninit;
use uninit::uninit_array;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Shash {
    hash: [u8; 32],
    path: PathBuf,
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
    type Error = io::Error;

    fn try_from(path: &Path) -> io::Result<Self> {
        let mut hasher = Sha256::new();
        hasher.update(path.as_os_str().as_encoded_bytes());
        hasher.update([0u8]);
        hasher.update(path.len().to_le_bytes());
        hasher.update([0u8]);
        let mut file = BufReader::new(File::open(path)?);
        let mut total_len = 0usize;
        Ok(loop {
            match file.read_into_uninit(uninit_array![u8; 1024].as_out()) {
                Ok([]) => {
                    hasher.update([0u8]);
                    hasher.update(total_len.to_le_bytes());
                    break Self {
                        path: path.into(),
                        hash: hasher.finalize().into(),
                    };
                }
                Ok(buf) => {
                    total_len += buf.len();
                    hasher.update(buf);
                }
                Err(err) if err.kind() == ErrorKind::Interrupted => {}
                Err(err) => Err(err)?,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let path = Path::new("test_res/non_existent");
        assert_eq!(
            Shash::try_from(path).unwrap_err().kind(),
            ErrorKind::NotFound
        );
    }

    #[test]
    fn shash_display_test() {
        let path = Path::new("test_res/c/run");
        assert_eq!(
            &Shash::try_from(path).unwrap().to_string(),
            "jA3Ho3HqbKueuegDF0AXqC/hpuEJkBYKix73B3HQji0 [\"test_res/c/run\"]"
        );
    }
}
