// Pure domain model for the `tasks` feature.
//
// All side effects live in `infrastructure.rs`. Aggregates expose pure
// command functions that take state + input and return `(new_state, events)`.

use crate::task_management::shared::AppError;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct TaskId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct TaskTitle(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Task {
    pub id: TaskId,
    pub title: TaskTitle,
    pub completed: bool,
}

const UUID_LEN: usize = 36;

impl TaskId {
    pub fn parse(raw: &str) -> Result<Self, AppError> {
        if raw.len() != UUID_LEN {
            return Err(AppError::Validation(format!("invalid TaskId: {raw}")));
        }
        Ok(Self(raw.to_owned()))
    }
}

impl TaskTitle {
    pub fn parse(raw: &str) -> Result<Self, AppError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(AppError::Validation("title must not be empty".to_owned()));
        }
        if trimmed.len() > 200 {
            return Err(AppError::Validation("title must be <= 200 chars".to_owned()));
        }
        Ok(Self(trimmed.to_owned()))
    }
}

pub fn complete(task: Task) -> Result<Task, AppError> {
    if task.completed {
        return Err(AppError::State(format!("task {} already completed", task.id.0)));
    }
    Ok(Task { completed: true, ..task })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_title_rejects_empty() {
        assert!(TaskTitle::parse("   ").is_err());
    }

    #[test]
    fn complete_idempotency_is_an_error() {
        let task = Task {
            id: TaskId("1f8b2a02-1111-4222-8333-444455556666".into()),
            title: TaskTitle("buy milk".into()),
            completed: true,
        };
        assert!(complete(task).is_err());
    }
}
