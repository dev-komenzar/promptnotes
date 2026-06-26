//! Pure application layer for `update-feed-filter`. 副作用ゼロ (C-UF6)。
//!
//! `apply` のシグネチャ `(NoteFeed, UpdateFeedFilterCommand) -> NoteFeed` は
//! Repository / EventBus 等の port を取らない。これにより read model / 揮発 / no-event を
//! type-level に保証する (TP-SE1)。

use crate::note_feed::shared::types::{FeedFilter, NoteFeed, NormalizedQuery};

use super::domain::UpdateFeedFilterCommand;

pub struct UpdateFeedFilterUseCase;

impl UpdateFeedFilterUseCase {
    /// `workflows/update-feed-filter.md#steps` の DMMF pipeline を pattern match で表現。
    pub fn apply(feed: NoteFeed, cmd: UpdateFeedFilterCommand) -> NoteFeed {
        let filter = feed.filter().clone();
        let updated_filter = match cmd {
            UpdateFeedFilterCommand::SetQuery { raw } => {
                // I-F1 / C-UF1 / C-UF2: NFC + lowercase + 空文字降格
                filter.with_query(NormalizedQuery::from_raw(&raw))
            }
            UpdateFeedFilterCommand::SetDateRange { range } => filter.with_date_range(range),
            UpdateFeedFilterCommand::SetTag { tag } => filter.with_tag(tag),
            // C-UF4 / I-F6: 全リセット = 初期状態
            UpdateFeedFilterCommand::ClearAll => FeedFilter::initial(),
        };
        feed.with_filter(updated_filter)
    }
}
