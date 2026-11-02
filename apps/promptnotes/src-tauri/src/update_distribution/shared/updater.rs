//! Tauri v2 updater plugin adapter (`UpdaterPort` の production impl)。
//!
//! 自動確認 (`check-for-updates`) と手動確認 (`check-for-updates-manually`) が同じ取得経路を
//! 共有する (workflows/check-for-updates-manually.md#dependencies)。plugin の version 比較は
//! permissive な `version_comparator` で無効化し、比較は domain (`release_check::compare`) が行う (I-U2)。

use tauri_plugin_updater::UpdaterExt;

use super::ports::{RawRelease, UpdaterPort};
use super::types::UpdateError;

/// Tauri v2 updater plugin adapter implementing `UpdaterPort`.
///
/// Wraps `tauri::AppHandle` and uses `tauri_plugin_updater::UpdaterExt` to
/// fetch the latest available release.
pub struct TauriUpdaterPort {
    pub app_handle: tauri::AppHandle,
}

impl UpdaterPort for TauriUpdaterPort {
    fn fetch_latest_release(&self) -> Result<RawRelease, UpdateError> {
        // Build updater with permissive comparator so the plugin always returns
        // the remote release regardless of version. Our domain layer does the
        // real comparison (I-U2).
        let builder = self
            .app_handle
            .updater_builder()
            .version_comparator(|_current, _remote| true);

        // E2E test seam (s14 / s23 / s24 / s25): debug build では更新確認の取得先を local server に向けられる。
        // release build では分岐ごと compile out され、本番 endpoint (tauri.conf.json) は不変。
        #[cfg(debug_assertions)]
        let builder = with_test_endpoint(builder).map_err(|e| match e {
            TestEndpointError::InvalidUrl => UpdateError::ParseError,
            TestEndpointError::Apply => UpdateError::NetworkError,
        })?;

        let updater = builder.build().map_err(|e| {
            log::warn!("failed to build updater: {e:?}");
            UpdateError::NetworkError
        })?;

        // Bridge async -> sync: we're inside a Tauri async command (tokio runtime),
        // so block_in_place allows us to run the async check() synchronously.
        let check_result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(updater.check())
        });

        match check_result {
            Ok(Some(update)) => Ok(RawRelease {
                // `update.download_url` は latest.json の platform 別アセット URL
                // (tauri-action が api.github.com/.../releases/assets/<id> を書く) で
                // ブラウザで開くとバイナリ / JSON になる。toast の「View release」は
                // GitHub Releases page を開く契約 (aggregates.md#update-channel-aggregate
                // `url: Url (GitHub Releases page)`) なので、version から組み立てる。
                url: release_page_url(&update.version),
                version_string: update.version,
                notes: update.body.unwrap_or_default(),
            }),
            Ok(None) => {
                // 204 No Content: no release available on the remote.
                log::warn!("updater returned no release (204 No Content)");
                Err(UpdateError::NetworkError)
            }
            Err(e) => {
                log::warn!("updater check failed: {e:?}");
                Err(map_updater_error(e))
            }
        }
    }
}

/// debug build 限定 seam の取得先上書きに失敗した理由。呼出側が自分のエラー型に写す。
#[cfg(debug_assertions)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TestEndpointError {
    InvalidUrl,
    Apply,
}

/// E2E test seam: `TAURI_TEST_UPDATER_ENDPOINT` が設定されていれば updater の取得先をそれに差し替える。
///
/// 更新確認 (`check-for-updates` / `check-for-updates-manually`) と install (`install-update`) が
/// 同じ取得先を使うよう、両方の adapter がこの関数を通す。`#[cfg(debug_assertions)]` 限定で、
/// release build には存在しない (本番 endpoint は tauri.conf.json のみ)。
#[cfg(debug_assertions)]
pub(crate) fn with_test_endpoint(
    builder: tauri_plugin_updater::UpdaterBuilder,
) -> Result<tauri_plugin_updater::UpdaterBuilder, TestEndpointError> {
    let Ok(endpoint) = std::env::var("TAURI_TEST_UPDATER_ENDPOINT") else {
        return Ok(builder);
    };
    let url = tauri::Url::parse(&endpoint).map_err(|e| {
        log::warn!("invalid TAURI_TEST_UPDATER_ENDPOINT ({endpoint}): {e:?}");
        TestEndpointError::InvalidUrl
    })?;
    builder.endpoints(vec![url]).map_err(|e| {
        log::warn!("failed to apply TAURI_TEST_UPDATER_ENDPOINT: {e:?}");
        TestEndpointError::Apply
    })
}

/// GitHub Releases の tag page URL prefix。tag は release workflow が `v<version>` で切る。
const RELEASE_TAG_URL_PREFIX: &str = "https://github.com/dev-komenzar/promptnotes/releases/tag/v";

/// latest.json の `version` (例: `0.2.3` / `v0.2.3`) から GitHub Releases page の URL を組み立てる。
pub(crate) fn release_page_url(version: &str) -> String {
    let version = version.trim();
    let version = version.strip_prefix('v').unwrap_or(version);
    format!("{RELEASE_TAG_URL_PREFIX}{version}")
}

/// Maps `tauri_plugin_updater::Error` to our domain `UpdateError`.
fn map_updater_error(e: tauri_plugin_updater::Error) -> UpdateError {
    use tauri_plugin_updater::Error as E;

    match &e {
        // Network/IO/connection errors
        E::Reqwest(_) | E::Network(_) | E::Io(_) | E::Http(_) | E::EmptyEndpoints => {
            UpdateError::NetworkError
        }
        // Server said no release
        E::ReleaseNotFound => UpdateError::NetworkError,
        // Parse/format errors
        E::Semver(_) | E::Serialization(_) | E::UrlParse(_) | E::FormatDate => {
            UpdateError::ParseError
        }
        // Platform/target errors
        E::TargetNotFound(_) | E::TargetsNotFound(_) | E::UnsupportedArch | E::UnsupportedOs => {
            UpdateError::ParseError
        }
        // Everything else — log and default to NetworkError
        _ => {
            log::warn!("unmapped updater error: {e:?}");
            UpdateError::NetworkError
        }
    }
}
