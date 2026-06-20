use crate::errors::{NoteBodyError, NoteIdError, TagError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NoteId(String);

impl NoteId {
    pub fn try_from_str(raw: &str) -> Result<Self, NoteIdError> {
        if raw.len() == 14 && raw.chars().all(|c| c.is_ascii_digit()) {
            Ok(Self(raw.to_string()))
        } else {
            Err(NoteIdError::InvalidFormat(raw.to_string()))
        }
    }

    pub fn from_timestamp(ts: Timestamp) -> Self {
        let fmt = time::macros::format_description!(
            "[year][month][day][hour][minute][second]"
        );
        Self(ts.0.format(&fmt).expect("Timestamp formatting cannot fail"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteBody(String);

impl NoteBody {
    pub fn try_from_string(raw: String) -> Result<Self, NoteBodyError> {
        if raw.lines().any(|line| line.trim_end() == "---") {
            Err(NoteBodyError::ContainsFrontmatterDelimiter)
        } else {
            Ok(Self(raw))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag(String);

impl Tag {
    const FORBIDDEN: &'static [char] = &[' ', '\t', '\n', ',', '[', ']'];

    pub fn try_from_string(raw: &str) -> Result<Self, TagError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(TagError::EmptyAfterTrim);
        }
        if let Some(c) = trimmed.chars().find(|c| Self::FORBIDDEN.contains(c)) {
            return Err(TagError::InvalidChar(c));
        }
        Ok(Self(trimmed.to_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TagSet(Vec<Tag>);

impl TagSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_iter<I: IntoIterator<Item = Tag>>(iter: I) -> Self {
        let mut out: Vec<Tag> = Vec::new();
        for t in iter {
            if !out.contains(&t) {
                out.push(t);
            }
        }
        Self(out)
    }

    pub fn contains_name(&self, name: &str) -> bool {
        self.0.iter().any(|t| t.as_str() == name)
    }

    pub fn insert(&mut self, tag: Tag) -> bool {
        if self.0.contains(&tag) {
            false
        } else {
            self.0.push(tag);
            true
        }
    }

    pub fn remove(&mut self, name: &str) -> Option<Tag> {
        let idx = self.0.iter().position(|t| t.as_str() == name)?;
        Some(self.0.remove(idx))
    }

    pub fn as_slice(&self) -> &[Tag] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(OffsetDateTime);

impl Timestamp {
    pub fn from_offset_date_time(t: OffsetDateTime) -> Self {
        Self(t.replace_nanosecond(0).expect("zero nanos always valid"))
    }

    pub fn inner(&self) -> OffsetDateTime {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    id: NoteId,
    body: NoteBody,
    tags: TagSet,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl Note {
    pub fn create(body: NoteBody, tags: TagSet, now: Timestamp) -> Self {
        Self {
            id: NoteId::from_timestamp(now),
            body,
            tags,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn id(&self) -> &NoteId {
        &self.id
    }
    pub fn body(&self) -> &NoteBody {
        &self.body
    }
    pub fn tags(&self) -> &TagSet {
        &self.tags
    }
    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }
    pub fn updated_at(&self) -> Timestamp {
        self.updated_at
    }

    pub fn edit_body(mut self, new_body: NoteBody, now: Timestamp) -> Self {
        self.body = new_body;
        self.updated_at = now;
        self
    }

    pub fn assign_tag(mut self, tag: Tag, now: Timestamp) -> (Self, TagDiff) {
        if self.tags.insert(tag.clone()) {
            self.updated_at = now;
            (self, TagDiff::Added(tag))
        } else {
            (self, TagDiff::Unchanged)
        }
    }

    pub fn remove_tag(mut self, name: &str, now: Timestamp) -> (Self, TagDiff) {
        match self.tags.remove(name) {
            Some(removed) => {
                self.updated_at = now;
                (self, TagDiff::Removed(removed))
            }
            None => (self, TagDiff::Unchanged),
        }
    }

    pub fn body_for_clipboard(&self) -> String {
        self.body.as_str().to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagDiff {
    Unchanged,
    Added(Tag),
    Removed(Tag),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyDiff {
    Unchanged,
    Changed(NoteBody),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletedNote {
    pub id: NoteId,
    pub original_path: PathBuf,
}
