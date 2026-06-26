use std::path::PathBuf;

use super::note_id::NoteId;

/// 削除された Note の Undo ハンドル (domain/aggregates.md#notes-undo)。
/// identity を持たない短命な VO。`Note::delete_to_trash(self, original_path)`
/// が唯一の構築経路 (aggregate boundary 経由)。
///
/// application service が `Vec<DeletedNote>` の Undo スタックとして保持し、
/// 対応する Toast の有効期間中のみ復元可能 (I-N7)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletedNote {
    id: NoteId,
    original_path: PathBuf,
}

impl DeletedNote {
    /// `pub(crate)` で公開し、aggregate (Note::delete_to_trash) のみが構築する
    /// 設計を構造的に担保する。slice の application 層から直接呼ばれることはない。
    pub(crate) fn new(id: NoteId, original_path: PathBuf) -> Self {
        Self { id, original_path }
    }

    pub fn id(&self) -> &NoteId {
        &self.id
    }

    pub fn original_path(&self) -> &std::path::Path {
        &self.original_path
    }
}
