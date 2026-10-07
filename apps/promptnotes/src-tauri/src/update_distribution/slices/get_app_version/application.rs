//! Application layer for `get-app-version` slice.

use super::domain::{to_app_version, AppVersion, GetAppVersionQuery};

pub struct GetAppVersionUseCase;

impl GetAppVersionUseCase {
    /// C-GAV1: `Result` を露出しない。C-GAV4: 副作用なし。
    pub fn execute(_query: GetAppVersionQuery, raw_build_version: &str) -> AppVersion {
        to_app_version(raw_build_version)
    }
}
