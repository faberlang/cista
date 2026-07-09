//! Project `faber.lock` writer used by the package manager.
//!
//! `faber.lock` is a Faber build lockfile. Cista rewrites it after install; faber
//! consumes absolute paths from it without knowing about the package store.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const LOCK_FILE: &str = "faber.lock";

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
    fs::write(path, contents).map_err(|err| format!("failed to write {}: {err}", path.display()))
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

/// Build a lock record for a successfully installed Rust library.
pub fn locked_from_install(
    name: &str,
    version: &str,
    source_path: &Path,
    package_store_root: &Path,
    target_language: &str,
    target_triple: &str,
    artifact_name: &Path,
    crate_name: &str,
    rustc: &str,
    kind: &str,
) -> LockedPackage {
    let package_root = absolute_display(package_store_root);
    let target_dir = package_store_root
        .join("targets")
        .join(target_language)
        .join(target_triple);
    let artifact = target_dir.join(artifact_name);
    LockedPackage {
        name: name.to_owned(),
        version: version.to_owned(),
        source: format!("path:{}", absolute_display(source_path)),
        package_root: package_root.clone(),
        kind: kind.to_owned(),
        target_language: target_language.to_owned(),
        target_triple: target_triple.to_owned(),
        target_manifest: absolute_display(&target_dir.join("cista.toml")),
        interface_root: absolute_display(&package_store_root.join("interfaces")),
        artifact: absolute_display(&artifact),
        crate_name: crate_name.to_owned(),
        rustc: rustc.to_owned(),
    }
}

/// Resolve lock path inside a project root.
pub fn lock_path(project_root: &Path) -> PathBuf {
    project_root.join(LOCK_FILE)
}
