//! Package-store and runtime binding library for Faber.
//!
//! `cista` is intentionally library-first: high-level workflows in the `faber`
//! CLI should call into this crate, while the `cista` binary exposes low-level
//! inspection and maintenance commands for package plumbing.

pub mod cli;
pub mod commands;
pub mod credentials;

/// Install a package into the shared store (path or registry pin).
///
/// Product CLIs such as `faber` call this in-process rather than spawning the
/// `cista` binary. See `faber/docs/design/product-composition-radix-cista.md`.
pub use commands::install::run as install;
pub mod faber_lock;
pub mod manifest;
pub mod package;
pub mod project_manifest;
pub mod registry_http;
pub mod resolver;
pub mod runtime;
pub mod store;
pub mod target;
