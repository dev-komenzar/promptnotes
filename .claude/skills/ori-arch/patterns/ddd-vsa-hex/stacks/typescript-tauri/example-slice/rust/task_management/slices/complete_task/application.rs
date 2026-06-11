// Use-case orchestration for the `tasks` feature.
//
// Composes domain primitives + infrastructure ports into transactional
// operations. Kept separate from `commands.rs` so the use case is
// independently testable without a Tauri runtime.

use super::domain::{complete, Task, TaskId};
use super::infrastructure::TaskRepository;
use crate::task_management::shared::AppError;

pub fn complete_task_usecase(
    repo: &dyn TaskRepository,
    id: &TaskId,
) -> Result<Task, AppError> {
    let task = repo
        .find(id)
        .ok_or_else(|| AppError::NotFound(format!("task {} not found", id.0)))?;
    let next = complete(task)?;
    repo.save(next.clone());
    Ok(next)
}
