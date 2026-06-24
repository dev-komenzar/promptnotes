use std::fs;
use std::path::{Path, PathBuf};

use crate::note_capture::shared::ports::NoteRepository;
use crate::note_capture::shared::types::Note;

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
        let path = self
            .storage_dir
            .join(format!("{}.md", note.id().as_str()));
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
}
