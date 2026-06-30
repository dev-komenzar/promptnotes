use sha2::{Digest, Sha256};

/// SHA-256 hash of a NoteBody, used for external-change conflict detection
/// (spec: domain/aggregates.md#note-aggregate-elements I-N9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyHash(String);

impl BodyHash {
    pub fn from_body(body: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(body.as_bytes());
        Self(hex::encode(hasher.finalize()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
