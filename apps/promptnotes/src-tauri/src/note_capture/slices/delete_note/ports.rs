use std::path::Path;

use crate::note_capture::shared::types::DeletedNote;

/// OS ゴミ箱移動の失敗種別。spec.md#io-errors の最小集合 (PermissionDenied /
/// Io(String) / Unsupported) に固定。phase 7 finalize で OS 依存 adapter
/// (macOS NSWorkspace / Linux XDG trash / Windows SHFileOperation) を採用
/// する際に集合を拡張する場合は spec + test を同時更新する。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TrashErrorKind {
    #[error("permission denied")]
    PermissionDenied,
    #[error("trash io: {0}")]
    Io(String),
    #[error("trash not supported on this platform")]
    Unsupported,
}

/// OS のゴミ箱へファイルを移動する output port (I-DN2: 削除は trash 経由のみ)。
/// `std::fs::remove_file` 等の unlink API への直接依存を slice 内で禁止する
/// ための構造的境界。
pub trait TrashService {
    fn move_to_trash(&self, path: &Path) -> Result<(), TrashErrorKind>;
}

/// Undo スタック (`Vec<DeletedNote>`) を保持する application service の
/// 公開 port。slice からは `push` のみを呼ぶ (I-DN8: 既存要素を破壊しない、
/// pop / restore は restore-deleted-note slice の責務)。
pub trait UndoStack {
    fn push(&self, deleted: DeletedNote);
}
