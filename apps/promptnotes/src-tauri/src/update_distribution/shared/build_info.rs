//! ビルド時に埋め込まれる app version の単一情報源
//! (`workflows/get-app-version.md#notes`, I-U1)。

/// `check_for_updates` / `get_app_version` の両 command が使う (C-GAV5)。
pub fn build_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
