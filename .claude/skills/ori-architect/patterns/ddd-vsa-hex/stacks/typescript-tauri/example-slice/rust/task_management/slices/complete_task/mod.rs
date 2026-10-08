// Public API for the `tasks` feature.
//
// Anything not re-exported here is feature-internal. Cross-feature imports
// must go through this file; `commands::*` is the IPC surface re-exposed to
// the frontend via tauri-specta.

mod application;
mod commands;
mod domain;
mod infrastructure;

pub use commands::complete_task_cmd;
pub use domain::{Task, TaskId, TaskTitle};
