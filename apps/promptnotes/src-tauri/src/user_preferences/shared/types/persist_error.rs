//! Shared `PersistError` type for Settings 永続化失敗 (`settings.json` 書き出し失敗)。
//!
//! `update-settings` slice と `change-sort-order` slice の両方が参照する
//! 共通 error 型。従来は `UpdateSettingsError::PersistError` variant を
//! `change-sort-order` 側が `pub use` で再利用していたが、これだと
//! slice 間直接依存 (oq-cross-bc-import) が残る。`shared::types::PersistError`
//! に抽出することで両 slice が shared 層のみを参照する形に clean up した。

use std::path::PathBuf;

use thiserror::Error;

/// `settings.json` への永続化失敗を表す shared error type。
///
/// `SettingsRepository::save` が `io::Error` を返した際に wrap され、
/// どの path への書き出しが失敗したかを保持する。
#[derive(Debug, Error)]
#[error("failed to persist settings to {path:?}: {cause}")]
pub struct PersistError {
    /// 書き出し先の `settings.json` path。
    pub path: PathBuf,
    /// `SettingsRepository::save` が返した underlying I/O error。
    #[source]
    pub cause: std::io::Error,
}
