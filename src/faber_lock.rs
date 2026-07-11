//! Project `faber.lock` writer used by the package manager.
//!
//! `faber.lock` is a Faber build lockfile. Cista rewrites it after install; faber
//! consumes absolute paths from it without knowing about the package store.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const LOCK_FILE: &str = "faber.lock";
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaberLock {
    #[serde(default, rename = "package")]
    pub packages: Vec<LockedPackage>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub source: String,
    pub package_root: String,
    pub kind: String,
    pub target_language: String,
    pub target_triple: String,
    pub target_manifest: String,
    pub interface_root: String,
    /// Absolute artifact path when present; empty for interfaces-only packages.
    #[serde(default)]
    pub artifact: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub rustc: String,
}

/// Read a lockfile if present; missing file yields an empty lock.
pub fn read_lock(path: &Path) -> Result<FaberLock, String> {
    if !path.exists() {
        return Ok(FaberLock::default());
    }
    let contents = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    toml::from_str(&contents).map_err(|err| format!("invalid {}: {err}", path.display()))
}

/// Write a lockfile with stable package ordering.
pub fn write_lock(path: &Path, lock: &FaberLock) -> Result<(), String> {
    let mut ordered = lock.clone();
    ordered
        .packages
        .sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
    let contents = toml::to_string_pretty(&ordered)
        .map_err(|err| format!("failed to render {}: {err}", path.display()))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let temporary_path = temporary_lock_path(path);
    write_and_replace(path, &temporary_path, contents.as_bytes())
}

fn temporary_lock_path(path: &Path) -> PathBuf {
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .map_or_else(|| LOCK_FILE.into(), |name| name.to_os_string());
    let mut temporary_name = file_name;
    temporary_name.push(format!(".{}.{}.tmp", std::process::id(), sequence));
    path.with_file_name(temporary_name)
}

fn write_and_replace(path: &Path, temporary_path: &Path, contents: &[u8]) -> Result<(), String> {
    let mut temporary = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary_path)
        .map_err(|err| format!("failed to create {}: {err}", temporary_path.display()))?;
    let write_result = temporary
        .write_all(contents)
        .and_then(|()| temporary.sync_all())
        .map_err(|err| format!("failed to write {}: {err}", temporary_path.display()));
    drop(temporary);
    let result = write_result.and_then(|()| {
        fs::rename(temporary_path, path)
            .map_err(|err| format!("failed to replace {}: {err}", path.display()))
    });
    match result {
        Ok(()) => Ok(()),
        Err(operation_error) => match fs::remove_file(temporary_path) {
            Ok(()) => Err(operation_error),
            Err(cleanup_error) if cleanup_error.kind() == std::io::ErrorKind::NotFound => {
                Err(operation_error)
            }
            Err(cleanup_error) => Err(format!(
                "{operation_error}; failed to remove {}: {cleanup_error}",
                temporary_path.display()
            )),
        },
    }
}

/// Upsert one package record by name (exact version replace).
pub fn upsert_package(lock: &mut FaberLock, package: LockedPackage) {
    if let Some(existing) = lock
        .packages
        .iter_mut()
        .find(|entry| entry.name == package.name)
    {
        *existing = package;
        return;
    }
    lock.packages.push(package);
}

/// Absolute path helper for lock records.
pub fn absolute_display(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

/// Build a lock record for a successfully installed package.
///
/// When `has_artifact` is false (interfaces-only install), `artifact` is left
/// empty. The field is still serialized because `faber` treats it as part of
/// the lockfile schema and uses the empty string to distinguish source-only
/// interfaces from missing lock data.
pub struct InstalledLockInput<'a> {
    pub name: &'a str,
    pub version: &'a str,
    pub source_path: &'a Path,
    pub package_store_root: &'a Path,
    pub target_language: &'a str,
    pub target_triple: &'a str,
    pub artifact_name: &'a Path,
    pub crate_name: &'a str,
    pub rustc: &'a str,
    pub kind: &'a str,
    pub has_artifact: bool,
}

pub fn locked_from_install(input: InstalledLockInput<'_>) -> LockedPackage {
    let package_root = absolute_display(input.package_store_root);
    let target_dir = input
        .package_store_root
        .join("targets")
        .join(input.target_language)
        .join(input.target_triple);
    let artifact = if input.has_artifact && !input.artifact_name.as_os_str().is_empty() {
        absolute_display(&target_dir.join(input.artifact_name))
    } else {
        String::new()
    };
    LockedPackage {
        name: input.name.to_owned(),
        version: input.version.to_owned(),
        source: format!("path:{}", absolute_display(input.source_path)),
        package_root: package_root.clone(),
        kind: input.kind.to_owned(),
        target_language: input.target_language.to_owned(),
        target_triple: input.target_triple.to_owned(),
        target_manifest: absolute_display(&target_dir.join("cista.toml")),
        interface_root: absolute_display(&input.package_store_root.join("interfaces")),
        artifact,
        crate_name: input.crate_name.to_owned(),
        rustc: input.rustc.to_owned(),
    }
}

/// Resolve lock path inside a project root.
pub fn lock_path(project_root: &Path) -> PathBuf {
    project_root.join(LOCK_FILE)
}

#[cfg(test)]
#[path = "faber_lock_test.rs"]
mod tests;
