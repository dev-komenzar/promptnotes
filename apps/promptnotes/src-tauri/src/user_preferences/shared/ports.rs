use std::io;
use std::path::Path;

use super::types::{Settings, SettingsEvent, StorageDir};

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
///
/// # 契約 (`aggregates.md#settings-aggregate-invariants`)
///
/// impl は返す `StorageDir` が **I-S1 (絶対パス)** と **I-S2 (任意の妥当な
/// `config_path` を子孫として含まない)** の両方を満たすことを保証する責務を負う。
/// load-settings slice はこの契約を信頼して `default_storage_dir()` の戻り値に
/// 対し I-S2 を defensive re-check しない (見つけ次第 panic ではなく、port 契約違反は
/// 上位 (infrastructure テスト) で防ぐ)。
pub trait OsDirs {
    fn default_storage_dir(&self) -> StorageDir;
}

/// `Settings` の読み込み / 永続化を提供する port (`update-settings` slice 用)。
///
/// - `load`: 現在の `Settings` を返す。`load-settings` slice 通過後の状態を取得する想定。
/// - `save`: 更新後の `Settings` を `settings.json` に書き出す。失敗は `io::Error` で返す。
pub trait SettingsRepository {
    fn load(&self) -> Settings;
    fn save(&self, settings: &Settings) -> io::Result<()>;
}

/// User Preferences BC 内で発行される domain event の同期 in-process bus
/// (`domain-events.md#notes-sync-rationale`)。
pub trait EventBus {
    fn publish(&self, event: SettingsEvent);
}
