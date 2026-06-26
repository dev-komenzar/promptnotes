/// 期間絞り込み (`aggregates.md#note-feed-aggregate-elements`)。
///
/// `Custom { from, to }` の範囲整合性 (from <= to) は本 enum では強制しない
/// (spec.md#oq-date-range-validation で deferred)。
///
/// `serde` 表現は `#[serde(tag = "kind", rename_all = "snake_case")]` で
/// `{ "kind": "last_7_days" }` / `{ "kind": "custom", "from": "...", "to": "..." }`
/// に揃える (Tauri command surface 用)。
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DateRangeFilter {
    Last7Days,
    Last30Days,
    Last90Days,
    #[default]
    All,
    Custom {
        from: String,
        to: String,
    },
}
