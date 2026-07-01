//! Tauri command surface for the `update-feed-filter` slice.
//!
//! `UpdateFeedFilterUseCase::apply` を Tauri から呼び出せる薄い wrapper。
//! 副作用ゼロ (C-UF6) を踏襲し Repository / EventBus は inject しない。Tauri 側の
//! 唯一の副作用は `State<InMemoryNoteFeedState>` の `replace`。
//!
//! 4 variants (SetQuery / SetDateRange / SetTag / ClearAll) を 1 つの Tauri command
//! ([`update_feed_filter`]) で受ける。input は tagged union ([`UpdateFeedFilterInput`])
//! として serde で deserialize し、`UpdateFeedFilterCommand` に lower する。
//! Tag::new での I-N6 validation 失敗のみ surface (`TagErrorDto`)。

use serde::Deserialize;
use std::sync::Arc;

use tauri::State;

use super::application::UpdateFeedFilterUseCase;
use super::domain::UpdateFeedFilterCommand;
use crate::note_capture::shared::types::{Tag, TagError};
use crate::note_feed::shared::adapters::InMemoryNoteFeedState;
use crate::note_feed::shared::types::{DateRangeFilter, FeedFilter};
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UpdateFeedFilterInput {
    SetQuery {
        raw: String,
    },
    SetDateRange {
        range: DateRangeFilter,
    },
    /// `raw = None` で tag filter を解除。
    SetTag {
        raw: Option<String>,
    },
    ClearAll,
}

#[derive(Debug, serde::Serialize)]
pub struct NoteFeedFilterDto {
    pub query: Option<String>,
    pub date_range: DateRangeFilter,
    pub tag: Option<String>,
}

impl From<&FeedFilter> for NoteFeedFilterDto {
    fn from(f: &FeedFilter) -> Self {
        Self {
            query: f.query().map(|q| q.as_str().to_string()),
            date_range: f.date_range().clone(),
            tag: f.tag().map(|t| t.name().to_string()),
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UpdateFeedFilterErrorDto {
    /// Tag::new での I-N6 (禁止文字 / 空文字) 違反。
    InvalidTag { raw: String, reason: String },
    /// `DateRangeFilter::Custom { from, to }` の `from > to` 違反
    /// (smart constructor / `validate()` で reject)。
    InvalidDateRange { reason: String },
}

impl From<(String, TagError)> for UpdateFeedFilterErrorDto {
    fn from((raw, e): (String, TagError)) -> Self {
        let reason = match e {
            TagError::InvalidChar { .. } => "invalid_char".to_string(),
            TagError::Empty => "empty".to_string(),
        };
        Self::InvalidTag { raw, reason }
    }
}

impl From<crate::note_feed::shared::types::DateRangeFilterError> for UpdateFeedFilterErrorDto {
    fn from(e: crate::note_feed::shared::types::DateRangeFilterError) -> Self {
        use crate::note_feed::shared::types::DateRangeFilterError;
        let reason = match e {
            DateRangeFilterError::FromAfterTo { .. } => "from_after_to".to_string(),
        };
        Self::InvalidDateRange { reason }
    }
}

fn lower(
    input: UpdateFeedFilterInput,
) -> Result<UpdateFeedFilterCommand, UpdateFeedFilterErrorDto> {
    Ok(match input {
        UpdateFeedFilterInput::SetQuery { raw } => UpdateFeedFilterCommand::SetQuery { raw },
        UpdateFeedFilterInput::SetDateRange { range } => {
            range.validate().map_err(UpdateFeedFilterErrorDto::from)?;
            UpdateFeedFilterCommand::SetDateRange { range }
        }
        UpdateFeedFilterInput::SetTag { raw } => {
            let tag = match raw {
                Some(s) => Some(Tag::new(&s).map_err(|e| UpdateFeedFilterErrorDto::from((s, e)))?),
                None => None,
            };
            UpdateFeedFilterCommand::SetTag { tag }
        }
        UpdateFeedFilterInput::ClearAll => UpdateFeedFilterCommand::ClearAll,
    })
}

#[tauri::command]
pub async fn update_feed_filter(
    feed_state: State<'_, Arc<InMemoryNoteFeedState>>,
    input: UpdateFeedFilterInput,
) -> Result<NoteFeedFilterDto, UpdateFeedFilterErrorDto> {
    let cmd = lower(input)?;
    let feed = feed_state.snapshot();
    let updated = UpdateFeedFilterUseCase::apply(feed, cmd);
    let dto = NoteFeedFilterDto::from(updated.filter());
    feed_state.replace(updated);
    Ok(dto)
}
