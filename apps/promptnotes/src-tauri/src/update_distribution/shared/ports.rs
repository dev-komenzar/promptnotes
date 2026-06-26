use super::types::{NewVersionDetected, UpdateError};

/// Parse 前の生 GitHub Releases payload。`UpdaterPort` の戻り型。
/// `Version::from_str` での parse は use case 側の責務 (`workflows/check-for-updates.md#steps` step 2)。
#[derive(Debug, Clone)]
pub struct RawRelease {
    pub version_string: String,
    pub url: String,
    pub notes: String,
}

/// Tauri v2 updater plugin の薄ラッパー port (`workflows/check-for-updates.md#dependencies`)。
///
/// production impl (`TauriUpdaterPort`) は release infrastructure 整備 (ori-6l4) 完了後に
/// 別 follow-up issue で追加する。本 slice では `FakeUpdater` (test) のみ実装。
pub trait UpdaterPort {
    fn fetch_latest_release(&self) -> Result<RawRelease, UpdateError>;
}

/// Update Distribution BC の domain event 同期 in-process bus
/// (`domain-events.md#notes-sync-rationale`)。
pub trait EventBus {
    fn publish(&self, event: NewVersionDetected);
}
