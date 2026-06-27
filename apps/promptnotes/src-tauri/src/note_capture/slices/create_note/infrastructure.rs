use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::note_capture::shared::ports::NoteRepository;
use crate::note_capture::shared::types::{Note, NoteBody, NoteId, Tag, TagSet, Timestamp};

/// Filesystem implementation of [`NoteRepository`].
///
/// Writes `<storage_dir>/<note_id>.md` containing a YAML frontmatter
/// (`createdAt`, `updatedAt`, `tags`) followed by the body. The directory is
/// created lazily on the first write. Concurrency is the caller's
/// responsibility — Tauri commands serialize on the async runtime by default.
pub struct FsNoteRepository {
    storage_dir: PathBuf,
}

impl FsNoteRepository {
    pub fn new(storage_dir: PathBuf) -> Self {
        Self { storage_dir }
    }
}

impl NoteRepository for FsNoteRepository {
    fn write(&self, note: &Note) -> std::io::Result<()> {
        fs::create_dir_all(&self.storage_dir)?;
        let path = self.storage_dir.join(format!("{}.md", note.id().as_str()));
        let tags_inline = note
            .tags()
            .as_slice()
            .iter()
            .map(|t| t.name())
            .collect::<Vec<_>>()
            .join(", ");
        let mut content = String::new();
        content.push_str("---\n");
        content.push_str(&format!(
            "createdAt: {}\n",
            note.created_at().format_yyyymmddhhmmss()
        ));
        content.push_str(&format!(
            "updatedAt: {}\n",
            note.updated_at().format_yyyymmddhhmmss()
        ));
        content.push_str(&format!("tags: [{tags_inline}]\n"));
        content.push_str("---\n");
        content.push_str(note.body().as_str());
        fs::write(&path, content)
    }

    fn storage_dir(&self) -> &Path {
        &self.storage_dir
    }

    fn load_by_id(&self, id: &NoteId) -> io::Result<Option<Note>> {
        let path = self.storage_dir.join(format!("{}.md", id.as_str()));
        let raw = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        parse_note_md(&raw).map(Some).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("malformed note at {}: {e}", path.display()),
            )
        })
    }

    fn list_all(&self) -> io::Result<Vec<Note>> {
        let entries = match fs::read_dir(&self.storage_dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };

        let mut notes = Vec::new();
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(err) => {
                    log::warn!("note_repository.list_all: read_dir entry error: {err}");
                    continue;
                }
            };
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            let raw = match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(err) => {
                    log::warn!(
                        "note_repository.list_all: read_to_string failed for {}: {err}",
                        path.display()
                    );
                    continue;
                }
            };
            match parse_note_md(&raw) {
                Ok(note) => notes.push(note),
                Err(err) => {
                    log::warn!(
                        "note_repository.list_all: parse_note_md failed for {}: {err}",
                        path.display()
                    );
                }
            }
        }
        Ok(notes)
    }
}

#[derive(Debug, thiserror::Error)]
enum ParseError {
    #[error("missing opening frontmatter delimiter")]
    MissingOpenDelimiter,
    #[error("missing closing frontmatter delimiter")]
    MissingCloseDelimiter,
    #[error("missing required frontmatter key '{0}'")]
    MissingKey(&'static str),
    #[error("invalid timestamp for '{key}': {raw}")]
    InvalidTimestamp { key: &'static str, raw: String },
    #[error("invalid tag '{0}'")]
    InvalidTag(String),
    #[error("invalid body: {0}")]
    InvalidBody(String),
}

/// Parse the on-disk `.md` file format produced by [`FsNoteRepository::write`].
/// Strict: every persisted file is also a NoteBody (no `---` line in the body
/// region), so any malformed input becomes `io::ErrorKind::InvalidData` at the
/// trait boundary.
fn parse_note_md(raw: &str) -> Result<Note, ParseError> {
    let mut lines = raw.split_inclusive('\n');
    let opener = lines.next().ok_or(ParseError::MissingOpenDelimiter)?;
    if opener.trim_end_matches('\n') != "---" {
        return Err(ParseError::MissingOpenDelimiter);
    }

    let mut created_at: Option<Timestamp> = None;
    let mut updated_at: Option<Timestamp> = None;
    let mut tags: Option<TagSet> = None;
    let mut closing_seen = false;
    let mut body_start = 0usize;
    body_start += opener.len();

    for line in lines.by_ref() {
        body_start += line.len();
        let stripped = line.trim_end_matches('\n');
        if stripped == "---" {
            closing_seen = true;
            break;
        }
        let (key, value) = match stripped.split_once(':') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };
        match key {
            "createdAt" => {
                created_at = Some(Timestamp::parse_yyyymmddhhmmss(value).map_err(|_| {
                    ParseError::InvalidTimestamp {
                        key: "createdAt",
                        raw: value.to_string(),
                    }
                })?);
            }
            "updatedAt" => {
                updated_at = Some(Timestamp::parse_yyyymmddhhmmss(value).map_err(|_| {
                    ParseError::InvalidTimestamp {
                        key: "updatedAt",
                        raw: value.to_string(),
                    }
                })?);
            }
            "tags" => {
                tags = Some(parse_tags_inline(value)?);
            }
            _ => {}
        }
    }

    if !closing_seen {
        return Err(ParseError::MissingCloseDelimiter);
    }

    let created_at = created_at.ok_or(ParseError::MissingKey("createdAt"))?;
    let updated_at = updated_at.ok_or(ParseError::MissingKey("updatedAt"))?;
    let tags = tags.ok_or(ParseError::MissingKey("tags"))?;
    let body_str = &raw[body_start..];
    let body =
        NoteBody::new(body_str.to_string()).map_err(|e| ParseError::InvalidBody(e.to_string()))?;

    Ok(Note::from_persisted(body, tags, created_at, updated_at))
}

fn parse_tags_inline(raw: &str) -> Result<TagSet, ParseError> {
    let inner = raw
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| ParseError::InvalidTag(raw.to_string()))?;
    let parsed = inner
        .split(',')
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(|t| Tag::new(t).map_err(|_| ParseError::InvalidTag(t.to_string())))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TagSet::from_tags(parsed))
}
