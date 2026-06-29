use time::format_description::FormatItem;
use time::macros::format_description;
use time::Date;

const ISO_DATE: &[FormatItem<'static>] = format_description!("[year]-[month]-[day]");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
pub enum FeedDateError {
    #[error("date string '{0}' does not match YYYY-MM-DD")]
    InvalidFormat(String),
}

/// Note Feed BC の日付 VO (`aggregates.md#note-feed-aggregate-elements` の
/// `DateRangeFilter::Custom { from, to }` 由来)。`time::Date` (day-precision、
/// timezone 無し) の newtype で、wire format は ISO 8601 date `YYYY-MM-DD`。
///
/// `Timestamp` (Note Capture BC、second-precision `OffsetDateTime`) とは粒度が
/// 異なる。Note Feed の date range filter は日単位の絞り込みが UI 契約
/// (date picker) のため、本 VO で day-precision を型レベルで表現する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FeedDate(Date);

impl FeedDate {
    pub fn from_date(date: Date) -> Self {
        Self(date)
    }

    pub fn as_date(&self) -> Date {
        self.0
    }

    pub fn parse_iso(s: &str) -> Result<Self, FeedDateError> {
        Date::parse(s, ISO_DATE)
            .map(Self)
            .map_err(|_| FeedDateError::InvalidFormat(s.to_string()))
    }

    pub fn to_iso(&self) -> String {
        self.0
            .format(ISO_DATE)
            .expect("ISO date formatting must not fail for a valid Date")
    }
}

impl std::fmt::Display for FeedDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_iso())
    }
}

impl serde::Serialize for FeedDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_iso())
    }
}

impl<'de> serde::Deserialize<'de> for FeedDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FeedDate::parse_iso(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::{FeedDate, FeedDateError};

    #[test]
    fn parse_iso_accepts_yyyy_mm_dd() {
        let d = FeedDate::parse_iso("2026-01-15").expect("valid ISO date");
        assert_eq!(d.to_iso(), "2026-01-15");
    }

    #[test]
    fn parse_iso_rejects_rfc3339_datetime() {
        let err = FeedDate::parse_iso("2026-01-15T00:00:00Z").expect_err("must reject");
        assert!(matches!(err, FeedDateError::InvalidFormat(_)));
    }

    #[test]
    fn parse_iso_rejects_garbage() {
        assert!(FeedDate::parse_iso("not a date").is_err());
        assert!(FeedDate::parse_iso("").is_err());
        assert!(FeedDate::parse_iso("2026/01/15").is_err());
    }

    #[test]
    fn ordering_is_date_natural_order() {
        let a = FeedDate::parse_iso("2026-01-01").unwrap();
        let b = FeedDate::parse_iso("2026-02-01").unwrap();
        assert!(a < b);
        assert!(b > a);
        assert_eq!(a, FeedDate::parse_iso("2026-01-01").unwrap());
    }

    #[test]
    fn serde_roundtrip_preserves_iso_string() {
        let d = FeedDate::parse_iso("2026-06-29").unwrap();
        let json = serde_json::to_string(&d).expect("serialize");
        assert_eq!(json, r#""2026-06-29""#);
        let back: FeedDate = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, d);
    }

    #[test]
    fn serde_rejects_non_iso_format() {
        let result: Result<FeedDate, _> = serde_json::from_str(r#""2026-01-01T00:00:00Z""#);
        assert!(result.is_err(), "RFC 3339 datetime must be rejected");
    }
}
