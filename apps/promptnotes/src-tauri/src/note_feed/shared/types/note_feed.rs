use super::FeedFilter;

/// Note Feed BC の唯一の集約 root (`aggregates.md#note-feed-aggregate`)。read model、揮発。
///
/// 本 slice (`update-feed-filter`) では filter 軸のみを扱う。
/// `sort` / `source` は後続 slice (`change-sort-order` / `list-feed`) で完成させる前提のため、
/// 現段階では filter のみ field として持つ最小構造。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NoteFeed {
    filter: FeedFilter,
}

impl NoteFeed {
    /// I-F6 の起動時初期状態 (filter 空)。
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn filter(&self) -> &FeedFilter {
        &self.filter
    }

    /// FeedFilter を差し替えた新しい NoteFeed を返す (move semantics)。
    pub fn with_filter(mut self, filter: FeedFilter) -> Self {
        self.filter = filter;
        self
    }
}
