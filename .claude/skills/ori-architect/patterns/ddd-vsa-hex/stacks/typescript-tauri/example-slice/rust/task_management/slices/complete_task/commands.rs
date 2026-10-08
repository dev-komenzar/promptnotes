// Tauri command surface for the `complete-task` slice.
//
// Commands marked with `#[tauri::command]` + `#[specta::specta]` are picked
// up by `tauri-specta` and re-exported to TypeScript at
// `apps/<app>/src/task-management/shared/ipc/bindings.ts`
// (see .ori/architecture.md cross_root).

use super::application::complete_task_usecase;
use super::domain::{Task, TaskId};
use super::infrastructure::MemoryRepo;
use crate::task_management::shared::AppError;

#[tauri::command]
#[specta::specta]
pub fn complete_task_cmd(id: String) -> Result<Task, AppError> {
    let parsed = TaskId::parse(&id)?;
    let repo = MemoryRepo::default();
    complete_task_usecase(&repo, &parsed)
}
