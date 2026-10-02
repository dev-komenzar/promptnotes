//! Production `EventBus` adapter for the Note Capture BC.
//!
//! Tauri の `app.emit` 経由で UI 側へ `note:*` イベントを通知する
//! (`user_preferences::shared::adapters::event_bus` と同型)。emit 失敗
//! (window がまだ無い等) は無視して握り潰す: 永続化は `NoteRepository` /
//! `TrashService` の責務であり、通知失敗でロールバックはしない。
//!
//! External file change events (`NoteFile*Externally`) は note-feed BC の
//! `detect_external_changes::commands::AppEventBus` が購読・中継するため、
//! 本 adapter の対象外 (`to_tauri_event` は `None` を返す)。

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime};

use crate::note_capture::shared::events::DomainEvent;
use crate::note_capture::shared::ports::EventBus;
use crate::note_capture::shared::types::TagSet;

pub const NOTE_CREATED_EVENT: &str = "note:created";
pub const NOTE_BODY_EDITED_EVENT: &str = "note:body_edited";
pub const NOTE_TAGS_CHANGED_EVENT: &str = "note:tags_changed";
pub const NOTE_DELETED_TO_TRASH_EVENT: &str = "note:deleted_to_trash";
pub const NOTE_RESTORED_FROM_TRASH_EVENT: &str = "note:restored_from_trash";

#[derive(Serialize)]
struct NoteCreatedPayload {
    note_id: String,
    created_at: String,
    initial_tags: Vec<String>,
}

#[derive(Serialize)]
struct NoteBodyEditedPayload {
    note_id: String,
    updated_at: String,
}

#[derive(Serialize)]
struct NoteTagsChangedPayload {
    note_id: String,
    tags: Vec<String>,
    updated_at: String,
}

#[derive(Serialize)]
struct NoteDeletedToTrashPayload {
    note_id: String,
    original_path: String,
    deleted_at: String,
}

#[derive(Serialize)]
struct NoteRestoredFromTrashPayload {
    note_id: String,
    restored_at: String,
}

fn tag_names(tags: &TagSet) -> Vec<String> {
    tags.as_slice()
        .iter()
        .map(|t| t.name().to_string())
        .collect()
}

fn json<T: Serialize>(payload: T) -> Value {
    serde_json::to_value(payload).expect("note event payload must serialize to JSON")
}

/// Maps a domain event to its Tauri event name + JSON payload.
/// Returns `None` for events this adapter does not surface (external file
/// change events; see module doc).
pub fn to_tauri_event(event: DomainEvent) -> Option<(&'static str, Value)> {
    let mapped = match event {
        DomainEvent::NoteCreated {
            note_id,
            created_at,
            initial_tags,
        } => (
            NOTE_CREATED_EVENT,
            json(NoteCreatedPayload {
                note_id: note_id.as_str().to_string(),
                created_at: created_at.format_rfc3339(),
                initial_tags: tag_names(&initial_tags),
            }),
        ),
        DomainEvent::NoteBodyEdited {
            note_id,
            updated_at,
        } => (
            NOTE_BODY_EDITED_EVENT,
            json(NoteBodyEditedPayload {
                note_id: note_id.as_str().to_string(),
                updated_at: updated_at.format_rfc3339(),
            }),
        ),
        DomainEvent::NoteTagsChanged {
            note_id,
            tags,
            updated_at,
        } => (
            NOTE_TAGS_CHANGED_EVENT,
            json(NoteTagsChangedPayload {
                note_id: note_id.as_str().to_string(),
                tags: tag_names(&tags),
                updated_at: updated_at.format_rfc3339(),
            }),
        ),
        DomainEvent::NoteDeletedToTrash {
            note_id,
            original_path,
            deleted_at,
        } => (
            NOTE_DELETED_TO_TRASH_EVENT,
            json(NoteDeletedToTrashPayload {
                note_id: note_id.as_str().to_string(),
                original_path: original_path.display().to_string(),
                deleted_at: deleted_at.format_rfc3339(),
            }),
        ),
        DomainEvent::NoteRestoredFromTrash {
            note_id,
            restored_at,
        } => (
            NOTE_RESTORED_FROM_TRASH_EVENT,
            json(NoteRestoredFromTrashPayload {
                note_id: note_id.as_str().to_string(),
                restored_at: restored_at.format_rfc3339(),
            }),
        ),
        DomainEvent::NoteFileCreatedExternally { .. }
        | DomainEvent::NoteFileModifiedExternally { .. }
        | DomainEvent::NoteFileDeletedExternally { .. } => return None,
    };
    Some(mapped)
}

pub struct TauriEventBus<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriEventBus<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> EventBus for TauriEventBus<R> {
    fn publish(&self, event: DomainEvent) {
        if let Some((name, payload)) = to_tauri_event(event) {
            let _ = self.app.emit(name, payload);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::*;
    use crate::note_capture::shared::types::{NoteId, Timestamp};

    fn ts() -> Timestamp {
        Timestamp::parse_yyyymmddhhmmss("20260102030405").unwrap()
    }

    #[test]
    fn note_deleted_to_trash_maps_to_named_event_with_payload() {
        let id = NoteId::from_timestamp(ts());
        let (name, payload) = to_tauri_event(DomainEvent::NoteDeletedToTrash {
            note_id: id.clone(),
            original_path: PathBuf::from("/notes/20260102030405.md"),
            deleted_at: ts(),
        })
        .unwrap();

        assert_eq!(name, NOTE_DELETED_TO_TRASH_EVENT);
        assert_eq!(
            payload,
            json!({
                "note_id": id.as_str(),
                "original_path": "/notes/20260102030405.md",
                "deleted_at": ts().format_rfc3339(),
            })
        );
    }

    #[test]
    fn note_restored_from_trash_maps_to_named_event_with_payload() {
        let id = NoteId::from_timestamp(ts());
        let (name, payload) = to_tauri_event(DomainEvent::NoteRestoredFromTrash {
            note_id: id.clone(),
            restored_at: ts(),
        })
        .unwrap();

        assert_eq!(name, NOTE_RESTORED_FROM_TRASH_EVENT);
        assert_eq!(
            payload,
            json!({
                "note_id": id.as_str(),
                "restored_at": ts().format_rfc3339(),
            })
        );
    }

    #[test]
    fn external_file_events_are_not_surfaced() {
        let event = DomainEvent::NoteFileDeletedExternally {
            note_id: NoteId::from_timestamp(ts()),
            file_path: PathBuf::from("/notes/20260102030405.md"),
            detected_at: ts(),
        };
        assert!(to_tauri_event(event).is_none());
    }
}
