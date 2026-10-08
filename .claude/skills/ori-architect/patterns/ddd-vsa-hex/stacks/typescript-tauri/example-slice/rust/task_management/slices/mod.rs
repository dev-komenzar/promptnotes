// Slice aggregator for the `task_management` BC.
//
// Each slice is a `kind: slice` module: its public surface is `mod.rs`,
// cross-slice direct imports are prohibited by `.ori/architecture.md`.
pub mod complete_task;
