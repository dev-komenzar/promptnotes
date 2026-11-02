//! 最新リリースの parse + 比較 (pure)。
//!
//! 自動確認 (`check-for-updates`) と手動確認 (`check-for-updates-manually`) の共通 step
//! (`workflows/check-for-updates.md#steps` / `workflows/check-for-updates-manually.md#steps`
//! の step 2〜3)。I-U2 の比較はここが単一エントリポイント。

use std::cmp::Ordering;

use super::ports::RawRelease;
use super::types::{Release, UpdateError, Version};

/// `compareVersions` step の結果。
/// `NewVersion(Release)` 以外は I-U2 によって `latest_release: None` に正規化される。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comparison {
    NewVersion(Release),
    UpToDate,
    OlderVersion,
}

/// `RawRelease` を parse して `Release` を作る (step 2)。
pub fn parse_release(raw: RawRelease) -> Result<Release, UpdateError> {
    let version = Version::from_str(&raw.version_string)?;
    Ok(Release::new(version, raw.url, raw.notes))
}

/// `(Version, Release)` から `Comparison` を導出する (step 3、I-U2)。
pub fn compare(current: &Version, candidate: Release) -> Comparison {
    match candidate.version().cmp(current) {
        Ordering::Greater => Comparison::NewVersion(candidate),
        Ordering::Equal => Comparison::UpToDate,
        Ordering::Less => Comparison::OlderVersion,
    }
}
