use std::path::Path;

use crate::note_capture::shared::types::{DeletedNote, NoteId};

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

/// OS ゴミ箱との双方向 I/O port。
///
/// - `move_to_trash`: delete-note slice が使用 (I-DN2: 削除は trash 経由のみ)
/// - `restore_from_trash`: restore-deleted-note slice が使用 (Undo 経路)
///
/// 両 method を 1 つの trait に集約する設計は spec
/// `.ori/slices/restore-deleted-note/spec.md#oq-trash-service-extension` の
/// open question として記録済。SRP より「OS trash の inverse op を同じ抽象に
/// まとめる」UI 契約優位を選択。
pub trait TrashService {
    fn move_to_trash(&self, path: &Path) -> Result<(), TrashErrorKind>;
    fn restore_from_trash(&self, path: &Path) -> Result<(), TrashErrorKind>;
}

/// Application service の Undo スタック (`Vec<DeletedNote>`) を抽象化する port。
///
/// - `push`: delete-note slice が使用 (I-DN8: 既存要素破壊なし)
/// - `find_by_id` / `remove_by_id`: restore-deleted-note slice が使用
///   (per-toast 独立性: 指定 NoteId 1 件のみを操作)
pub trait UndoStack {
    fn push(&self, deleted: DeletedNote);
    fn find_by_id(&self, id: &NoteId) -> Option<DeletedNote>;
    /// 指定 `NoteId` の DeletedNote を 1 件除去する。除去された要素を返す
    /// (audit / future 検証用、本 slice の application 層では discard)。
    fn remove_by_id(&self, id: &NoteId) -> Option<DeletedNote>;
}
