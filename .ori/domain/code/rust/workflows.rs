use crate::errors::*;
use crate::events::*;
use crate::note::*;
use crate::note_feed::*;
use crate::settings::*;
use crate::update_channel::*;
use std::path::PathBuf;

pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}

pub trait NoteRepository: Send + Sync {
    fn write(&self, note: &Note) -> Result<(), PersistError>;
    fn read(&self, id: &NoteId) -> Result<Note, ReadError>;
    fn delete_path(&self, id: &NoteId) -> PathBuf;
}

pub trait TrashService: Send + Sync {
    fn move_to_trash(&self, path: &std::path::Path) -> Result<(), TrashError>;
    fn restore_from_trash(&self, path: &std::path::Path) -> Result<(), TrashError>;
}

pub trait ClipboardService: Send + Sync {
    fn write(&self, text: &str) -> Result<(), ClipboardError>;
}

pub trait SettingsRepository: Send + Sync {
    fn load(&self, config_path: &std::path::Path) -> Option<Settings>;
    fn persist(&self, config_path: &std::path::Path, settings: &Settings)
        -> Result<(), PersistError>;
}

pub trait OsDirs: Send + Sync {
    fn config_dir(&self) -> PathBuf;
    fn default_notes_dir(&self) -> PathBuf;
}

pub trait UpdaterPlugin: Send + Sync {
    fn fetch_latest(&self) -> Result<Release, UpdateError>;
}

pub trait UndoSlot: Send + Sync {
    fn replace(&self, deleted: DeletedNote);
    fn take(&self) -> Option<DeletedNote>;
    fn clear(&self);
}

// ----- Workflow input / output types -----

#[derive(Debug, Clone)]
pub struct CreateNoteCommand {
    pub raw_body: String,
    pub raw_tags: Vec<String>,
}

#[derive(Debug)]
pub enum CreateNoteError {
    InvalidTag {
        name: String,
        reason: TagError,
    },
    InvalidBody(NoteBodyError),
    PersistError(PersistError),
}

#[derive(Debug, Clone)]
pub struct AutoSaveNoteCommand {
    pub note_id: NoteId,
    pub new_body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushTrigger {
    BlockBlur,
    WindowBlur,
    AppQuit,
}

#[derive(Debug, Clone)]
pub struct FlushNoteCommand {
    pub note_id: NoteId,
    pub pending_body: String,
    pub trigger: FlushTrigger,
}

#[derive(Debug)]
pub enum EditNoteError {
    NoteNotFound(NoteNotFound),
    InvalidBody(NoteBodyError),
    PersistError(PersistError),
}

#[derive(Debug, Clone)]
pub struct AssignTagCommand {
    pub note_id: NoteId,
    pub raw_tag: String,
}

#[derive(Debug)]
pub enum AssignTagError {
    NoteNotFound(NoteNotFound),
    InvalidTag {
        name: String,
        reason: TagError,
    },
    PersistError(PersistError),
}

#[derive(Debug, Clone)]
pub struct RemoveTagCommand {
    pub note_id: NoteId,
    pub tag_name: String,
}

#[derive(Debug)]
pub enum RemoveTagError {
    NoteNotFound(NoteNotFound),
    PersistError(PersistError),
}

#[derive(Debug, Clone)]
pub struct DeleteNoteCommand {
    pub note_id: NoteId,
}

#[derive(Debug)]
pub enum DeleteNoteError {
    NoteNotFound(NoteNotFound),
    TrashError(TrashError),
}

#[derive(Debug, Clone)]
pub struct RestoreDeletedNoteCommand;

#[derive(Debug)]
pub enum RestoreDeletedNoteError {
    NoUndoAvailable(NoUndoAvailable),
    TrashRestoreError(TrashError),
    ReadError(ReadError),
}

#[derive(Debug, Clone)]
pub struct CopyNoteBodyCommand {
    pub note_id: NoteId,
}

#[derive(Debug)]
pub enum CopyNoteBodyError {
    NoteNotFound(NoteNotFound),
    ClipboardError(ClipboardError),
}

#[derive(Debug, Clone)]
pub enum UpdateFeedFilterCommand {
    SetQuery { raw: String },
    SetDateRange { range: DateRangeFilter },
    SetTag { tag: Option<Tag> },
    ClearAll,
}

#[derive(Debug, Clone)]
pub struct ChangeSortOrderCommand {
    pub new_sort: SortOrder,
}

#[derive(Debug)]
pub enum ChangeSortOrderError {
    PersistError(PersistError),
}

#[derive(Debug, Clone)]
pub struct UpdateSettingsCommand {
    pub new_storage_dir: Option<PathBuf>,
    pub new_theme: Option<Theme>,
}

#[derive(Debug)]
pub enum UpdateSettingsError {
    InvalidPath(InvalidPath),
    PersistError(PersistError),
}

#[derive(Debug, Clone)]
pub struct LoadSettingsCommand {
    pub config_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CheckForUpdatesCommand {
    pub current_version: Version,
}

// ----- Workflow trait signatures -----
//
// Each workflow is a function: deps -> (input -> Result<Output, Error>).
// In production these are implemented as struct methods that hold deps,
// but the trait form documents the type signature precisely.

pub trait CreateNoteWorkflow {
    fn run(&self, cmd: CreateNoteCommand) -> Result<(Note, NoteCreated), CreateNoteError>;
}

pub trait AutoSaveNoteWorkflow {
    fn run(
        &self,
        cmd: AutoSaveNoteCommand,
    ) -> Result<Option<(Note, NoteBodyEdited)>, EditNoteError>;
}

pub trait FlushNoteWorkflow {
    fn run(
        &self,
        cmd: FlushNoteCommand,
    ) -> Result<Option<(Note, NoteBodyEdited)>, EditNoteError>;
}

pub trait AssignTagWorkflow {
    fn run(
        &self,
        cmd: AssignTagCommand,
    ) -> Result<Option<(Note, NoteTagsChanged)>, AssignTagError>;
}

pub trait RemoveTagWorkflow {
    fn run(
        &self,
        cmd: RemoveTagCommand,
    ) -> Result<Option<(Note, NoteTagsChanged)>, RemoveTagError>;
}

pub trait DeleteNoteWorkflow {
    fn run(
        &self,
        cmd: DeleteNoteCommand,
    ) -> Result<(DeletedNote, NoteDeletedToTrash), DeleteNoteError>;
}

pub trait RestoreDeletedNoteWorkflow {
    fn run(
        &self,
        cmd: RestoreDeletedNoteCommand,
    ) -> Result<(Note, NoteRestoredFromTrash), RestoreDeletedNoteError>;
}

pub trait CopyNoteBodyWorkflow {
    fn run(&self, cmd: CopyNoteBodyCommand) -> Result<(), CopyNoteBodyError>;
}

pub trait UpdateFeedFilterWorkflow<'a> {
    fn run(
        &self,
        feed: NoteFeed<'a>,
        cmd: UpdateFeedFilterCommand,
    ) -> NoteFeed<'a>;
}

pub trait ChangeSortOrderWorkflow {
    fn run(
        &self,
        cmd: ChangeSortOrderCommand,
    ) -> Result<SortPreferenceChanged, ChangeSortOrderError>;
}

pub trait UpdateSettingsWorkflow {
    fn run(
        &self,
        cmd: UpdateSettingsCommand,
    ) -> Result<(Settings, SettingsDiff, Vec<DomainEvent>), UpdateSettingsError>;
}

pub trait LoadSettingsWorkflow {
    fn run(&self, cmd: LoadSettingsCommand) -> Settings;
}

pub trait CheckForUpdatesWorkflow {
    fn run(
        &self,
        cmd: CheckForUpdatesCommand,
    ) -> Result<UpdateChannel, UpdateError>;
}
