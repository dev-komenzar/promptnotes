/// 期間絞り込み (`aggregates.md#note-feed-aggregate-elements`)。
///
/// `Custom { from, to }` の範囲整合性 (from <= to) は本 enum では強制しない
/// (spec.md#oq-date-range-validation で deferred)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
