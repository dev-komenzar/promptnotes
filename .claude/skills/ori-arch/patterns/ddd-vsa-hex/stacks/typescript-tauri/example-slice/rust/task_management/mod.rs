// Public API for the `task_management` bounded context.
//
// Each child (`shared`, `slices::*`) lives below this module. Cross-BC
// imports must go through `crate::shared::contracts` (the app-level
// `<root.path>/shared/`); reaching into `task_management::*` from a sibling
// BC is forbidden by `cross_bc.via` in .ori/architecture.md.
pub mod shared;
pub mod slices;
