use serde::Serialize;
use specta::Type;
use thiserror::Error;

#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("validation: {0}")]
    Validation(String),
    #[error("state: {0}")]
    State(String),
    #[error("not_found: {0}")]
    NotFound(String),
}

pub type AppResult<T> = Result<T, AppError>;
