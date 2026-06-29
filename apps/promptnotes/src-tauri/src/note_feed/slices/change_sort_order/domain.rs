//! Domain types for `change-sort-order` slice.
//!
//! `ChangeSortOrderError` は `shared::types::PersistError` の alias (ori-hpo.8)。
//! 従来は `UpdateSettingsError` を `pub use` していたが、これだと slice 間
//! 直接依存 (oq-cross-bc-import) が残るため、shared 層に抽出した `PersistError`
//! を直接参照する形に変更した。本 slice は validation を行わないため
//! `InvalidPath` variant は構造上発生せず、`PersistError` のみで十分。

use crate::user_preferences::shared::types::SortOrder;

/// `change-sort-order` slice の input (`workflows/change-sort-order.md#input`)。
#[derive(Debug, Clone)]
pub struct ChangeSortOrderCommand {
    pub new_sort: SortOrder,
}

/// Slice のエラー型。`shared::types::PersistError` を再利用 (ori-hpo.8 / C-CSO6)。
///
/// `PersistError { path, cause }` が「settings.json 書き出し失敗」を表す
/// (`update-settings` slice と共通の shared error type)。
pub use crate::user_preferences::shared::types::PersistError as ChangeSortOrderError;
