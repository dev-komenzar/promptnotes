use std::cmp::Ordering;
use std::str::FromStr;

use super::UpdateError;

/// strict semver (`MAJOR.MINOR.PATCH` + optional pre-release / build metadata) を表す VO
/// (`aggregates.md#update-channel-aggregate-elements`)。
///
/// `semver` crate による strict semver parse を行う。pre-release (`-rc1`) や
/// build metadata (`+sha`) を含む文字列も正しく parse する
/// (spec.md#oq-version-pre-release: ori-2lm.9 で strict semver 対応に拡張)。
///
/// 比較順序は `semver` crate 1.x の実装に従う:
/// - pre-release 版は対応する release 版より小さい (`0.4.0-rc1 < 0.4.0`)
/// - build metadata は `semver` 2.0 仕様では比較無視だが、1.x crate では
///   比較対象に含まれる (異なる build metadata → 異なる `Ord` 結果になる可能性あり)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version(semver::Version);

impl Version {
    /// `"0.3.1"` / `"0.4.0-rc1"` / `"0.4.0+sha"` を strict semver として parse。
    /// semver 仕様に合致しない文字列は `UpdateError::ParseError` を返す。
    /// `<Version as FromStr>::from_str` への薄い委譲。
    ///
    /// 同名 inherent 提供は ergonomics 優先 (call site で `use std::str::FromStr` を要求しない)。
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, UpdateError> {
        <Self as FromStr>::from_str(s)
    }
}

impl FromStr for Version {
    type Err = UpdateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        semver::Version::parse(s)
            .map(Self)
            .map_err(|_| UpdateError::ParseError)
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
