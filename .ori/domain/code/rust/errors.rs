use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TagError {
    #[error("tag becomes empty after trim")]
    EmptyAfterTrim,
    #[error("tag contains forbidden character: {0:?}")]
    InvalidChar(char),
}

#[derive(Debug, Error)]
#[error("Note not found: {id}")]
pub struct NoteNotFound {
    pub id: String,
}

#[derive(Debug, Error)]
#[error("Persist failed at {path}: {source}")]
pub struct PersistError {
    pub path: PathBuf,
    #[source]
    pub source: std::io::Error,
}

#[derive(Debug, Error)]
#[error("Read failed at {path}: {source}")]
pub struct ReadError {
    pub path: PathBuf,
    #[source]
    pub source: std::io::Error,
}

#[derive(Debug, Error)]
pub enum TrashError {
    #[error("OS trash move failed at {path}: {source}")]
    MoveFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("OS trash restore failed at {path}: {source}")]
    RestoreFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("Clipboard write failed: {reason}")]
    WriteFailed { reason: String },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PathError {
    #[error("path is not absolute: {0}")]
    NotAbsolute(PathBuf),
}

#[derive(Debug, Error)]
#[error("Invalid path {path}: {reason}")]
pub struct InvalidPath {
    pub path: PathBuf,
    pub reason: PathError,
}

#[derive(Debug, Error)]
#[error("No undo target available")]
pub struct NoUndoAvailable;

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("Network error during update check: {0}")]
    NetworkError(String),
    #[error("Failed to parse update response: {0}")]
    ParseError(String),
    #[error("GitHub API rate limited")]
    RateLimited,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NoteIdError {
    #[error("invalid NoteId format: expected YYYYMMDDhhmmss, got {0:?}")]
    InvalidFormat(String),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NoteBodyError {
    #[error("body must not contain the YAML frontmatter delimiter (---) at the start of a line")]
    ContainsFrontmatterDelimiter,
}

#[derive(Debug, Error, Clone)]
pub enum VersionError {
    #[error("invalid semver string: {0}")]
    InvalidSemver(String),
}
