use crate::note::{NoteId, TagSet, Timestamp};
use crate::note_feed::SortOrder;
use crate::settings::Theme;
use crate::update_channel::{Release, Version};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteCreated {
    pub note_id: NoteId,
    pub created_at: Timestamp,
    pub initial_tags: TagSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteBodyEdited {
    pub note_id: NoteId,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteTagsChanged {
    pub note_id: NoteId,
    pub tags: TagSet,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteDeletedToTrash {
    pub note_id: NoteId,
    pub original_path: PathBuf,
    pub deleted_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteRestoredFromTrash {
    pub note_id: NoteId,
    pub restored_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageDirChanged {
    pub old_dir: PathBuf,
    pub new_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeChanged {
    pub new_theme: Theme,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortPreferenceChanged {
    pub new_sort: SortOrder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewVersionDetected {
    pub current_version: Version,
    pub latest_release: Release,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    NoteCreated(NoteCreated),
    NoteBodyEdited(NoteBodyEdited),
    NoteTagsChanged(NoteTagsChanged),
    NoteDeletedToTrash(NoteDeletedToTrash),
    NoteRestoredFromTrash(NoteRestoredFromTrash),
    StorageDirChanged(StorageDirChanged),
    ThemeChanged(ThemeChanged),
    SortPreferenceChanged(SortPreferenceChanged),
    NewVersionDetected(NewVersionDetected),
}

pub trait EventBus {
    fn publish(&self, event: DomainEvent);
}
