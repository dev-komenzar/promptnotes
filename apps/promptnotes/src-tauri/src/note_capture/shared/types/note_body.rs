#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteBody(String);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum NoteBodyError {
    #[error("note body must not contain a frontmatter delimiter line '---'")]
    ContainsFrontmatterDelimiter,
}

impl NoteBody {
    pub fn new(raw: String) -> Result<Self, NoteBodyError> {
        if raw.lines().any(|l| l.trim_end() == "---") {
            return Err(NoteBodyError::ContainsFrontmatterDelimiter);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
