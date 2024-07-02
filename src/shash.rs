use base64ct::{Base64Unpadded, Encoding};
use nix::NixPath;
use sha2::{Digest, Sha512_256};
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
        let mut hasher = Sha512_256::new();
        hasher.update(path.as_os_str().as_encoded_bytes());
        hasher.update([0u8]);
        hasher.update(path.len().to_le_bytes());
        hasher.update([0u8]);
        let mut file = BufReader::new(File::open(path)?);
        let mut total_len = 0usize;
        loop {
            match file.read_into_uninit(uninit_array![u8; 1024].as_out()) {
                Ok([]) => {
                    hasher.update([0u8]);
                    hasher.update(total_len.to_le_bytes());
                    break Ok(Self {
                        path: path.into(),
                        hash: hasher.finalize().into(),
                    });
                }
                Ok(buf) => {
                    total_len += buf.len();
                    hasher.update(buf);
                }
                Err(err) if err.kind() == ErrorKind::Interrupted => {}
                Err(err) => break Err(err),
            }
        }
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
                    27, 71, 26, 187, 63, 147, 245, 247, 19, 51, 76, 49, 61, 10, 111, 254, 80, 125,
                    141, 73, 195, 219, 77, 157, 188, 235, 73, 136, 149, 249, 104, 111
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
            "G0cauz+T9fcTM0wxPQpv/lB9jUnD202dvOtJiJX5aG8 [\"test_res/c/run\"]"
        );
    }
}
