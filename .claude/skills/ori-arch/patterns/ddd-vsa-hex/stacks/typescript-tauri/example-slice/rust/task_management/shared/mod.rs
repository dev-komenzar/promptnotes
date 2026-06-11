// Cross-feature primitives.
//
// Lives below every feature module in the dependency graph. May not depend
// on any sibling under `features/`.

pub mod events;
pub mod result;

pub use events::DomainEvent;
pub use result::{AppError, AppResult};
