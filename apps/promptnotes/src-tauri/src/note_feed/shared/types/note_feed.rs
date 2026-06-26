use crate::user_preferences::shared::types::SortOrder;

use super::FeedFilter;

/// Note Feed BC の唯一の集約 root (`aggregates.md#note-feed-aggregate`)。read model、揮発。
///
/// `update-feed-filter` slice では filter 軸のみを扱い、`sort` / `source` は drop していた
/// (notes.md decision)。`change-sort-order` slice で `sort: SortOrder` field を復活させた。
///
/// `SortOrder` は `user_preferences::shared::types::SortOrder` を直接借りる
/// (Customer-Supplier 規約。Supplier = User Preferences、Customer = Note Feed)。
/// `change-sort-order` が「NoteFeed → Settings の唯一の逆流」を application service で扱う
/// ことで、両 aggregate の `sort` が同期される (aggregates.md#notes-sort-side-effect)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NoteFeed {
    filter: FeedFilter,
    sort: SortOrder,
}

impl NoteFeed {
    /// I-F6 の起動時初期状態 (filter 空 + sort default = {CreatedAt, Desc} per I-S3)。
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn filter(&self) -> &FeedFilter {
        &self.filter
    }

    pub fn sort(&self) -> SortOrder {
        self.sort
    }

    /// FeedFilter を差し替えた新しい NoteFeed を返す (move semantics)。
    pub fn with_filter(mut self, filter: FeedFilter) -> Self {
        self.filter = filter;
        self
    }

    /// `aggregates.md#note-feed-aggregate-operations` の `change_sort`。
    /// in-memory 反映のみ。Settings 永続化は `change-sort-order` slice の application service
    /// が同一トランザクションで担う (`#notes-sort-side-effect`)。
    pub fn change_sort(mut self, sort: SortOrder) -> Self {
        self.sort = sort;
        self
    }
}
