//! Domain types for `update-feed-filter` slice (pure).

use crate::note_capture::shared::types::Tag;
use crate::note_feed::shared::types::DateRangeFilter;

/// `update-feed-filter` slice の input (`workflows/update-feed-filter.md#input`)。
///
/// 4 分岐の sum type: 検索バー入力 / 期間プリセット選択 / タグチップ操作 / 全リセット。
/// `SetTag.tag = None` は tag filter 解除。`SetQuery.raw` の正規化は use case 側 (NormalizedQuery::from_raw) が担う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateFeedFilterCommand {
    SetQuery { raw: String },
    SetDateRange { range: DateRangeFilter },
    SetTag { tag: Option<Tag> },
    ClearAll,
}
