use time::format_description::well_known::Rfc3339;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};

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

    /// RFC 3339 / ISO 8601 wire representation for Tauri DTOs. JavaScript
    /// `Date.parse` accepts this form, unlike the compact `YYYYMMDDhhmmss`
    /// used for filenames and `NoteId`.
    pub fn format_rfc3339(&self) -> String {
        self.0
            .format(&Rfc3339)
            .expect("RFC 3339 formatting must not fail for a valid OffsetDateTime")
    }

    pub fn parse_yyyymmddhhmmss(s: &str) -> Result<Self, TimestampError> {
        // The format has no timezone component, so we parse as a primitive
        // datetime and pin it to UTC (matches `format_yyyymmddhhmmss`, which
        // discards the offset).
        PrimitiveDateTime::parse(s, YYYYMMDDHHMMSS)
            .map(|p| Self::from_offset_datetime(p.assume_offset(UtcOffset::UTC)))
            .map_err(|_| TimestampError::InvalidFormat(s.to_string()))
    }
}
