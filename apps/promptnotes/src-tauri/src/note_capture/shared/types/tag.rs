/// Forbidden characters that must not appear in a normalized tag name.
/// Per I-N6 / domain/aggregates.md#note-aggregate-elements.
pub const FORBIDDEN_TAG_CHARS: &[char] = &[' ', '\t', '\n', ',', '[', ']'];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tag(String);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TagError {
    #[error("tag '{raw}' contains invalid character")]
    InvalidChar { raw: String },
    #[error("tag must not be empty after normalization")]
    Empty,
}

impl Tag {
    pub fn new(raw: &str) -> Result<Self, TagError> {
        // I-N6 normalization: trim → forbidden-char check on the trimmed
        // content → lowercase. Surrounding whitespace is *normalized away*,
        // not a violation; only *interior* forbidden chars reject the tag.
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(TagError::Empty);
        }
        if trimmed.chars().any(|c| FORBIDDEN_TAG_CHARS.contains(&c)) {
            return Err(TagError::InvalidChar {
                raw: raw.to_string(),
            });
        }
        Ok(Self(trimmed.to_lowercase()))
    }

    pub fn name(&self) -> &str {
        &self.0
    }
}
