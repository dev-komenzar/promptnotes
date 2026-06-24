/// BC-wide error type. Slice-specific errors live inside `slices/<slice>/domain.rs`
/// and convert into this enum at the command boundary.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("note capture error: {0}")]
    NoteCapture(String),
}
