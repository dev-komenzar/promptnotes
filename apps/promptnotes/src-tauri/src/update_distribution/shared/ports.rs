use super::types::{NewVersionDetected, UpdateError, Version};

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
/// production impl は `check_for_updates::infrastructure::TauriUpdaterPort` (ori-2lm.8)。
pub trait UpdaterPort {
    fn fetch_latest_release(&self) -> Result<RawRelease, UpdateError>;
}

/// Update Distribution BC の domain event 同期 in-process bus
/// (`domain-events.md#notes-sync-rationale`)。
pub trait EventBus {
    fn publish(&self, event: NewVersionDetected);
}

/// 同一プロセス内で通知済みの `Release::version` を保持する揮発 state の port (I-U4)。
///
/// `claim` は「通知すべきか」の判定と「通知済みとして記録」を 1 操作で行う
/// (`check-for-updates` spec C-CFU8)。production impl は
/// `check_for_updates::infrastructure::InMemoryUpdateCheckState`。
pub trait NotifiedVersionLedger {
    /// `version` を通知すべきなら通知済みを `version` に更新して `true`。
    /// 通知済みバージョン以下なら state を変えずに `false`。
    fn claim(&self, version: &Version) -> bool;
}

impl<T: UpdaterPort + ?Sized> UpdaterPort for std::rc::Rc<T> {
    fn fetch_latest_release(&self) -> Result<RawRelease, UpdateError> {
        (**self).fetch_latest_release()
    }
}

impl<T: EventBus + ?Sized> EventBus for std::rc::Rc<T> {
    fn publish(&self, event: NewVersionDetected) {
        (**self).publish(event)
    }
}

impl<T: NotifiedVersionLedger + ?Sized> NotifiedVersionLedger for std::rc::Rc<T> {
    fn claim(&self, version: &Version) -> bool {
        (**self).claim(version)
    }
}

impl<T: NotifiedVersionLedger + ?Sized> NotifiedVersionLedger for std::sync::Arc<T> {
    fn claim(&self, version: &Version) -> bool {
        (**self).claim(version)
    }
}
