use crate::note_capture::shared::types::Tag;

use super::{DateRangeFilter, NormalizedQuery};

/// NoteFeed の filter (`aggregates.md#note-feed-aggregate-elements`)。
/// query / date_range / tag の 3 軸を AND 合成する (I-F4)。`initial` は起動時の空状態 (I-F6)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FeedFilter {
    query: Option<NormalizedQuery>,
    date_range: DateRangeFilter,
    tag: Option<Tag>,
}

impl FeedFilter {
    /// 起動時の空状態 (`{ query: None, date_range: All, tag: None }`、I-F6)。
    pub fn initial() -> Self {
        Self::default()
    }

    pub fn query(&self) -> Option<&NormalizedQuery> {
        self.query.as_ref()
    }

    pub fn date_range(&self) -> &DateRangeFilter {
        &self.date_range
    }

    pub fn tag(&self) -> Option<&Tag> {
        self.tag.as_ref()
    }

    /// SetQuery / SetDateRange / SetTag が呼び出す書換え用 helper。
    /// 各 setter は他軸を保持する (C-UF5 直交性)。
    pub fn with_query(mut self, q: Option<NormalizedQuery>) -> Self {
        self.query = q;
        self
    }

    pub fn with_date_range(mut self, r: DateRangeFilter) -> Self {
        self.date_range = r;
        self
    }

    pub fn with_tag(mut self, t: Option<Tag>) -> Self {
        self.tag = t;
        self
    }
}
