pub mod application;
pub mod commands;
pub mod domain;
pub mod ports;

#[cfg(test)]
mod tests;

pub use application::CopyNoteBodyUseCase;
pub use commands::copy_note_body;
pub use domain::{CopyNoteBodyCommand, CopyNoteBodyError};
pub use ports::{ClipboardErrorKind, ClipboardService};
