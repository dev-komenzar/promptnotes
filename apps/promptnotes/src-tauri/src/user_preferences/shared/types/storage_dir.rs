use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// `Note` の `.md` ファイル保存先。I-S1 (絶対パス) を smart constructor で保証する。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StorageDir(PathBuf);

impl StorageDir {
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

impl TryFrom<PathBuf> for StorageDir {
    type Error = InvalidPath;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        if value.is_absolute() {
            Ok(Self(value))
        } else {
            Err(InvalidPath::NotAbsolute(value))
        }
    }
}

#[derive(Debug, Error)]
pub enum InvalidPath {
    #[error("storage_dir must be absolute: {0:?}")]
    NotAbsolute(PathBuf),
}
