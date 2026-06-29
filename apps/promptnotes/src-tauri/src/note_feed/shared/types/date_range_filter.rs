use super::{FeedDate, FeedDateError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DateRangeFilterError {
    #[error("custom date range `from` ({from}) is after `to` ({to})")]
    FromAfterTo { from: FeedDate, to: FeedDate },
}

/// 期間絞り込み (`aggregates.md#note-feed-aggregate-elements`)。
///
/// `Custom { from, to }` は `FeedDate` (day-precision VO) で型強化済み。
/// `from <= to` 不変条件は [`DateRangeFilter::custom`] smart constructor で施行する。
/// serde deserialize 後の検証は [`DateRangeFilter::validate`] で行う
/// (Tauri command 境界の `lower()` で呼ぶ)。
///
/// `serde` 表現は variant ごとに明示 `#[serde(rename = ...)]` を付与して
/// `{ "kind": "last_7_days" }` / `{ "kind": "custom", "from": "2026-01-01", "to": "2026-01-31" }`
/// に揃える (Tauri command surface 用)。
///
/// `rename_all = "snake_case"` の自動変換は数字境界に underscore を入れない
/// (`Last7Days` → `last7_days`) ため、TS UI 側が送る `last_7_days` と
/// 不一致になり deserialize に失敗する。回帰防止のため variant 単位で固定する。
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum DateRangeFilter {
    #[serde(rename = "last_7_days")]
    Last7Days,
    #[serde(rename = "last_30_days")]
    Last30Days,
    #[serde(rename = "last_90_days")]
    Last90Days,
    #[default]
    #[serde(rename = "all")]
    All,
    #[serde(rename = "custom")]
    Custom {
        from: FeedDate,
        to: FeedDate,
    },
}

impl DateRangeFilter {
    /// `Custom { from, to }` を `from <= to` 不変条件下で構築する smart constructor。
    /// `from > to` の場合は [`DateRangeFilterError::FromAfterTo`] を返す。
    pub fn custom(from: FeedDate, to: FeedDate) -> Result<Self, DateRangeFilterError> {
        if from > to {
            return Err(DateRangeFilterError::FromAfterTo { from, to });
        }
        Ok(Self::Custom { from, to })
    }

    /// `Custom` variant の `from <= to` 不変条件を検証する。
    /// serde deserialize 後 (smart constructor を経由しない経路) の防衛検証として
    /// Tauri command 境界で呼ぶ。
    pub fn validate(&self) -> Result<(), DateRangeFilterError> {
        match self {
            DateRangeFilter::Custom { from, to } if from > to => {
                Err(DateRangeFilterError::FromAfterTo {
                    from: *from,
                    to: *to,
                })
            }
            _ => Ok(()),
        }
    }

    /// `Custom` variant の `from` / `to` を返す accessor (他 variant は `None`)。
    pub fn custom_range(&self) -> Option<(FeedDate, FeedDate)> {
        match self {
            DateRangeFilter::Custom { from, to } => Some((*from, *to)),
            _ => None,
        }
    }
}

/// `DateRangeFilter::Custom` を wire 文字列から直接構築する helper。
/// `from` / `to` を ISO date (`YYYY-MM-DD`) として parse し、`from <= to` を検証する。
/// UI 側から文字列で受け取るテスト / 一時経路向け。production の Tauri 経路は
/// serde deserialize → `validate()` のフローを使う。
impl DateRangeFilter {
    pub fn custom_from_iso(from: &str, to: &str) -> Result<Self, CustomFromIsoError> {
        let from = FeedDate::parse_iso(from).map_err(CustomFromIsoError::From)?;
        let to = FeedDate::parse_iso(to).map_err(CustomFromIsoError::To)?;
        Self::custom(from, to).map_err(CustomFromIsoError::Range)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CustomFromIsoError {
    #[error("`from` is not a valid ISO date: {0}")]
    From(FeedDateError),
    #[error("`to` is not a valid ISO date: {0}")]
    To(FeedDateError),
    #[error("date range invalid: {0}")]
    Range(DateRangeFilterError),
}

#[cfg(test)]
mod tests {
    use super::{CustomFromIsoError, DateRangeFilter, DateRangeFilterError, FeedDate};

    fn assert_roundtrip(variant: DateRangeFilter, expected_json: &str) {
        let serialized = serde_json::to_string(&variant).expect("serialize");
        assert_eq!(serialized, expected_json, "serialize mismatch");
        let deserialized: DateRangeFilter =
            serde_json::from_str(expected_json).expect("deserialize");
        assert_eq!(deserialized, variant, "deserialize mismatch");
    }

    #[test]
    fn wire_format_last_7_days_uses_underscored_digits() {
        assert_roundtrip(DateRangeFilter::Last7Days, r#"{"kind":"last_7_days"}"#);
    }

    #[test]
    fn wire_format_last_30_days_uses_underscored_digits() {
        assert_roundtrip(DateRangeFilter::Last30Days, r#"{"kind":"last_30_days"}"#);
    }

    #[test]
    fn wire_format_last_90_days_uses_underscored_digits() {
        assert_roundtrip(DateRangeFilter::Last90Days, r#"{"kind":"last_90_days"}"#);
    }

    #[test]
    fn wire_format_all_is_lowercase() {
        assert_roundtrip(DateRangeFilter::All, r#"{"kind":"all"}"#);
    }

    #[test]
    fn wire_format_custom_carries_from_and_to_as_iso_date() {
        let range = DateRangeFilter::custom_from_iso("2026-01-01", "2026-02-01").expect("valid");
        assert_roundtrip(
            range,
            r#"{"kind":"custom","from":"2026-01-01","to":"2026-02-01"}"#,
        );
    }

    #[test]
    fn deserialize_rejects_legacy_snake_case_without_digit_underscore() {
        let result: Result<DateRangeFilter, _> =
            serde_json::from_str(r#"{"kind":"last7_days"}"#);
        assert!(
            result.is_err(),
            "legacy `last7_days` wire format must be rejected to surface UI/Rust drift"
        );
    }

    // ===== smart constructor: from <= to validation =====

    #[test]
    fn custom_accepts_from_equal_to() {
        let d = FeedDate::parse_iso("2026-01-15").unwrap();
        let range = DateRangeFilter::custom(d, d).expect("from == to is valid");
        assert_eq!(range.custom_range(), Some((d, d)));
    }

    #[test]
    fn custom_accepts_from_before_to() {
        let from = FeedDate::parse_iso("2026-01-01").unwrap();
        let to = FeedDate::parse_iso("2026-01-31").unwrap();
        let range = DateRangeFilter::custom(from, to).expect("from < to is valid");
        assert_eq!(range.custom_range(), Some((from, to)));
    }

    #[test]
    fn custom_rejects_from_after_to() {
        let from = FeedDate::parse_iso("2026-02-01").unwrap();
        let to = FeedDate::parse_iso("2026-01-01").unwrap();
        let err = DateRangeFilter::custom(from, to).expect_err("from > to must be rejected");
        assert!(matches!(err, DateRangeFilterError::FromAfterTo { .. }));
    }

    // ===== validate(): post-deserialize defense =====

    #[test]
    fn validate_passes_for_valid_custom() {
        let range = DateRangeFilter::custom_from_iso("2026-01-01", "2026-01-31").unwrap();
        assert!(range.validate().is_ok());
    }

    #[test]
    fn validate_passes_for_non_custom_variants() {
        assert!(DateRangeFilter::All.validate().is_ok());
        assert!(DateRangeFilter::Last7Days.validate().is_ok());
    }

    #[test]
    fn validate_rejects_deserialized_from_after_to() {
        // serde deserialize は smart constructor を経由しないので from > to が通ってしまう。
        // validate() が防衛検査として機能する事を pin する。
        let json = r#"{"kind":"custom","from":"2026-02-01","to":"2026-01-01"}"#;
        let range: DateRangeFilter = serde_json::from_str(json).expect("deserialize succeeds");
        let err = range.validate().expect_err("validate must reject from > to");
        assert!(matches!(err, DateRangeFilterError::FromAfterTo { .. }));
    }

    // ===== custom_from_iso helper =====

    #[test]
    fn custom_from_iso_accepts_valid_pair() {
        let range = DateRangeFilter::custom_from_iso("2026-01-01", "2026-01-31").unwrap();
        assert!(matches!(range, DateRangeFilter::Custom { .. }));
    }

    #[test]
    fn custom_from_iso_rejects_invalid_from() {
        let err = DateRangeFilter::custom_from_iso("not-a-date", "2026-01-31").unwrap_err();
        assert!(matches!(err, CustomFromIsoError::From(_)));
    }

    #[test]
    fn custom_from_iso_rejects_invalid_to() {
        let err = DateRangeFilter::custom_from_iso("2026-01-01", "2026/02/01").unwrap_err();
        assert!(matches!(err, CustomFromIsoError::To(_)));
    }

    #[test]
    fn custom_from_iso_rejects_from_after_to() {
        let err = DateRangeFilter::custom_from_iso("2026-02-01", "2026-01-01").unwrap_err();
        assert!(matches!(err, CustomFromIsoError::Range(_)));
    }
}
