//! Application layer for `get-default-storage-dir` slice.

use super::domain::GetDefaultStorageDirQuery;
use crate::user_preferences::shared::ports::OsDirs;
use crate::user_preferences::shared::types::StorageDir;

pub struct GetDefaultStorageDirUseCase<O: OsDirs> {
    os_dirs: O,
}

impl<O: OsDirs> GetDefaultStorageDirUseCase<O> {
    pub fn new(os_dirs: O) -> Self {
        Self { os_dirs }
    }

    /// C-GDSD1: `Result` を露出しない。C-GDSD2: 副作用なし (`settings.json` の読み書き /
    /// `ensure_dir` / event 発行をしない)。
    pub fn execute(&self, _query: GetDefaultStorageDirQuery) -> StorageDir {
        self.os_dirs.default_storage_dir()
    }
}
