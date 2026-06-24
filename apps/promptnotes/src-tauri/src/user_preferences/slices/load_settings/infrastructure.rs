use std::fs;
use std::io;
use std::path::Path;

use crate::user_preferences::shared::ports::{FileSystem, OsDirs};
use crate::user_preferences::shared::types::StorageDir;

/// `std::fs` を直接叩く [`FileSystem`] 実装。
pub struct StdFileSystem;

impl FileSystem for StdFileSystem {
    fn try_read(&self, path: &Path) -> Option<String> {
        // 不在 / I/O 失敗を区別せず `None` に降格 (保守的、C-LS3 と整合)。
        fs::read_to_string(path).ok()
    }

    fn ensure_dir(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }
}

/// composition root が事前に解決した OS default を保持する shim。
///
/// 実際の OS dir 解決は Tauri の `app.path()` に任せ、本 BC は
/// 「予め解決された絶対パス」を受け取るだけにする。Q6 の Settings 独立性に整合。
pub struct FixedOsDirs {
    default: StorageDir,
}

impl FixedOsDirs {
    pub fn new(default: StorageDir) -> Self {
        Self { default }
    }
}

impl OsDirs for FixedOsDirs {
    fn default_storage_dir(&self) -> StorageDir {
        self.default.clone()
    }
}
