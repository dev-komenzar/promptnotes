use super::Version;

/// GitHub Releases から取得した release を表す VO (`aggregates.md#update-channel-aggregate-elements`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    version: Version,
    url: String,
    notes: String,
}

impl Release {
    pub fn new(version: Version, url: String, notes: String) -> Self {
        Self { version, url, notes }
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn notes(&self) -> &str {
        &self.notes
    }
}
