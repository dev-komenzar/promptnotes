/// 期間絞り込み (`aggregates.md#note-feed-aggregate-elements`)。
///
/// `Custom { from, to }` の範囲整合性 (from <= to) は本 enum では強制しない
/// (spec.md#oq-date-range-validation で deferred)。
///
/// `serde` 表現は variant ごとに明示 `#[serde(rename = ...)]` を付与して
/// `{ "kind": "last_7_days" }` / `{ "kind": "custom", "from": "...", "to": "..." }`
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
        from: String,
        to: String,
    },
}

#[cfg(test)]
mod tests {
    use super::DateRangeFilter;

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
    fn wire_format_custom_carries_from_and_to() {
        assert_roundtrip(
            DateRangeFilter::Custom {
                from: "2026-01-01T00:00:00Z".to_owned(),
                to: "2026-02-01T00:00:00Z".to_owned(),
            },
            r#"{"kind":"custom","from":"2026-01-01T00:00:00Z","to":"2026-02-01T00:00:00Z"}"#,
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
}
