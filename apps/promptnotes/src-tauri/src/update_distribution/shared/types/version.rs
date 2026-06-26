use std::cmp::Ordering;
use std::str::FromStr;

use super::UpdateError;

/// semver (`MAJOR.MINOR.PATCH`) を表す VO (`aggregates.md#update-channel-aggregate-elements`)。
///
/// 本 slice では `major.minor.patch` 3-tuple の lexicographic 比較のみをサポート。
/// pre-release (`-rc1`) / build metadata (`+sha`) は `ParseError` で reject する
/// (spec.md#oq-version-pre-release、YAGNI; 必要になったら follow-up で拡張)。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// `"0.3.1"` を parse。3 つの dot-separated digit 以外 (pre-release / build metadata 含む) は
    /// `UpdateError::ParseError` を返す。`<Version as FromStr>::from_str` への薄い委譲。
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
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(UpdateError::ParseError);
        }
        let parse = |x: &str| x.parse::<u32>().map_err(|_| UpdateError::ParseError);
        Ok(Self::new(parse(parts[0])?, parse(parts[1])?, parse(parts[2])?))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
