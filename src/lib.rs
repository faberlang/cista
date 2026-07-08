//! Package-store and runtime binding library for Faber.
//!
//! `cista` is intentionally library-first: high-level workflows in the `faber`
//! CLI should call into this crate, while the `cista` binary exposes low-level
//! inspection and maintenance commands for package plumbing.

pub mod cache;
pub mod cli;
pub mod commands;
pub mod diagnostics;
pub mod manifest;
pub mod package;
pub mod resolver;
pub mod runtime;
pub mod target;

/// Current architectural status of the crate.
pub const STATUS: &str = "check and local install implemented; other commands staged";
