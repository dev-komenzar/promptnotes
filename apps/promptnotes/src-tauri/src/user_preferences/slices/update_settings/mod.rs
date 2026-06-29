pub mod application;
pub mod commands;
pub mod domain;

#[cfg(test)]
mod tests;

pub use application::UpdateSettingsUseCase;
pub use domain::{PersistError, SettingsEvent, UpdateSettingsCommand, UpdateSettingsError};
