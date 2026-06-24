use std::collections::HashSet;

use super::tag::Tag;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TagSet(Vec<Tag>);

impl TagSet {
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    /// Insertion-ordered dedupe by `Tag::name` (first occurrence wins, I-N5).
    pub fn from_tags<I: IntoIterator<Item = Tag>>(iter: I) -> Self {
        let mut seen: HashSet<String> = HashSet::new();
        let mut out: Vec<Tag> = Vec::new();
        for tag in iter {
            if seen.insert(tag.name().to_string()) {
                out.push(tag);
            }
        }
        Self(out)
    }

    pub fn as_slice(&self) -> &[Tag] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
