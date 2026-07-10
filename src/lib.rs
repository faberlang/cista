//! Package-store and runtime binding library for Faber.
//!
//! `cista` is intentionally library-first: high-level workflows in the `faber`
//! CLI should call into this crate, while the `cista` binary exposes low-level
//! inspection and maintenance commands for package plumbing.

pub mod cache;
pub mod cli;
pub mod commands;
pub mod diagnostics;
pub mod faber_lock;
pub mod manifest;
pub mod package;
pub mod project_manifest;
pub mod registry_http;
pub mod resolver;
pub mod runtime;
pub mod store;
pub mod target;

/// Current architectural status of the crate.
pub const STATUS: &str = "check, install (rlib + interfaces-only), store inspect/remove, faber.lock rewrite; registry staged";
