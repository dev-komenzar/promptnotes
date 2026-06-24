use super::timestamp::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NoteId(String);

impl NoteId {
    pub fn from_timestamp(ts: Timestamp) -> Self {
        Self(ts.format_yyyymmddhhmmss())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
