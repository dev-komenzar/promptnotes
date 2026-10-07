//! Domain types for `get-app-version` slice (pure).

use crate::update_distribution::shared::types::Version;

/// `get-app-version` slice の input (`workflows/get-app-version.md#input`)。入力なし。
#[derive(Debug, Clone, Copy, Default)]
pub struct GetAppVersionQuery;

/// 表示用のアプリバージョン文字列 (`workflows/get-app-version.md#output`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppVersion(String);

impl AppVersion {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

/// `toAppVersion` step (`workflows/get-app-version.md#steps`)。
///
/// - parse 可能 → `Version` の Display に正規化 (C-GAV2)
/// - parse 不能 → raw 文字列をそのまま返す (C-GAV3)
pub fn to_app_version(raw: &str) -> AppVersion {
    match Version::from_str(raw) {
        Ok(v) => AppVersion(v.to_string()),
        Err(_) => AppVersion(raw.to_string()),
    }
}
