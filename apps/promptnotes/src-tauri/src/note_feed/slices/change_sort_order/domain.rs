//! Domain types for `change-sort-order` slice.
//!
//! **PersistError は update-settings の `UpdateSettingsError` を再利用** (C-CSO6)。
//! 新しい error enum を定義せず、`pub use` で 1 つの型に統一する。
//! UI 層が両 slice からの error を同一 handler で扱えるようにする。

use crate::user_preferences::shared::types::SortOrder;

/// `change-sort-order` slice の input (`workflows/change-sort-order.md#input`)。
#[derive(Debug, Clone)]
pub struct ChangeSortOrderCommand {
    pub new_sort: SortOrder,
}

/// Slice のエラー型。`UpdateSettingsError` を再利用 (C-CSO6)。
///
/// `UpdateSettingsError::PersistError { path, cause }` variant が
/// 「settings.json 書き出し失敗」を表す（update-settings と共通）。
/// 本 slice 自身が validation を行わないため `InvalidPath` variant は構造上発生し得ない
/// （type は alias なので variant として存在はする）。
pub use crate::user_preferences::slices::update_settings::domain::UpdateSettingsError
    as ChangeSortOrderError;
