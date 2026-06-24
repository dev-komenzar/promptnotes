use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;

const YYYYMMDDHHMMSS: &[FormatItem<'static>] = format_description!(
    "[year][month][day][hour padding:zero][minute padding:zero][second padding:zero]"
);

/// Second-precision wrapper over [`OffsetDateTime`]. Sub-second components are
/// truncated on construction to satisfy I-N2 (id ↔ createdAt roundtrip).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(OffsetDateTime);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TimestampError {
    #[error("timestamp string '{0}' does not match YYYYMMDDhhmmss")]
    InvalidFormat(String),
}

impl Timestamp {
    pub fn from_offset_datetime(dt: OffsetDateTime) -> Self {
        Self(dt.replace_nanosecond(0).expect("0 is a valid nanosecond"))
    }

    pub fn into_offset_datetime(self) -> OffsetDateTime {
        self.0
    }

    pub fn format_yyyymmddhhmmss(&self) -> String {
        self.0
            .format(YYYYMMDDHHMMSS)
            .expect("YYYYMMDDhhmmss formatting must not fail for a valid OffsetDateTime")
    }

    pub fn parse_yyyymmddhhmmss(s: &str) -> Result<Self, TimestampError> {
        OffsetDateTime::parse(s, YYYYMMDDHHMMSS)
            .map(Self::from_offset_datetime)
            .map_err(|_| TimestampError::InvalidFormat(s.to_string()))
    }
}
