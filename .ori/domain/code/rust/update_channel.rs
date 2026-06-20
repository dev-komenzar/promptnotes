use crate::errors::VersionError;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version(semver::Version);

impl Version {
    pub fn try_from_str(raw: &str) -> Result<Self, VersionError> {
        semver::Version::parse(raw)
            .map(Self)
            .map_err(|e| VersionError::InvalidSemver(e.to_string()))
    }

    pub fn as_semver(&self) -> &semver::Version {
        &self.0
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    pub version: Version,
    pub url: Url,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionComparison {
    NewVersion(Release),
    UpToDate,
    OlderVersion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateChannel {
    pub current_version: Version,
    pub latest_release: Option<Release>,
}

impl UpdateChannel {
    pub fn new(current_version: Version) -> Self {
        Self {
            current_version,
            latest_release: None,
        }
    }

    pub fn with_release(self, release: Release) -> Self {
        if release.version > self.current_version {
            Self {
                latest_release: Some(release),
                ..self
            }
        } else {
            Self {
                latest_release: None,
                ..self
            }
        }
    }

    pub fn has_new_version(&self) -> bool {
        self.latest_release.is_some()
    }
}
