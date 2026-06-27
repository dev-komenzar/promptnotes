//! Domain layer of slice `list-feed`.
//!
//! Spec: `.ori/slices/list-feed/spec.md#io-input`.
//! The command itself is argumentless; the application service resolves
//! `storage_dir` and the current `NoteFeed` snapshot from its context.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ListFeedCommand;
