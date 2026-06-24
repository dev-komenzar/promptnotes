use std::io;
use std::path::Path;

use super::types::StorageDir;

/// `settings.json` の読み取り / `storage_dir` の物理確保を提供する port。
///
/// - `try_read`: 不在 / 読み取り失敗を区別せず `None` に降格（C-LS2 / C-LS3 / 保守的扱い）。
/// - `ensure_dir`: 初回起動時の `storage_dir` 作成（C-LS5）。失敗は use case 層で silent に握り潰す（C-LS6）。
pub trait FileSystem {
    fn try_read(&self, path: &Path) -> Option<String>;
    fn ensure_dir(&self, path: &Path) -> io::Result<()>;
}

/// OS 慣習に基づく default `storage_dir` を返す port。
///
/// macOS / Linux / Windows ごとの慣習パスは infrastructure 実装に閉じ込める。
pub trait OsDirs {
    fn default_storage_dir(&self) -> StorageDir;
}
