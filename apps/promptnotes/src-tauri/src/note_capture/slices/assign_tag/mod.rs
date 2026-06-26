pub mod application;
pub mod commands;
pub mod domain;

#[cfg(test)]
mod tests;

pub use application::AssignTagUseCase;
pub use commands::assign_tag;
pub use domain::{AssignTagCommand, AssignTagError};
