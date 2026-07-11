//! Shared package-store layout helpers (cista-owned discovery).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::manifest::{read_manifest, CistaManifest, MANIFEST_FILE};

#[derive(Clone, Debug)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub package_root: PathBuf,
    pub interfaces_dir: PathBuf,
    pub targets_dir: PathBuf,
}

/// Resolve store root using explicit path, `CISTAE_HOME`, or default home.
pub fn store_root(explicit: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        return Ok(normalize_path(path));
    }
    if let Some(path) = env::var_os("CISTAE_HOME") {
        return Ok(normalize_path(Path::new(&path)));
    }
    let Some(home) = env::var_os("HOME") else {
        return Err(
            "CISTAE_HOME is not set and HOME is unavailable; pass --store explicitly".to_owned(),
        );
    };
    Ok(PathBuf::from(home).join(".faber").join("cistae"))
}

/// Normalize a path, canonicalizing when possible.
pub fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// List installed package versions under a store root.
pub fn list_installed(store_root: &Path) -> Result<Vec<InstalledPackage>, String> {
    let mut packages = Vec::new();
    if !store_root.exists() {
        return Ok(packages);
    }
    let package_entries = fs::read_dir(store_root).map_err(|err| {
        format!(
            "failed to read package store {}: {err}",
            store_root.display()
        )
    })?;
    for package_entry in package_entries {
        let package_entry = package_entry.map_err(|err| {
            format!(
                "failed to read package store entry under {}: {err}",
                store_root.display()
            )
        })?;
        let package_path = package_entry.path();
        if !package_entry
            .file_type()
            .map_err(|err| format!("failed to inspect {}: {err}", package_path.display()))?
            .is_dir()
        {
            continue;
        }
        let name = utf8_directory_name(&package_path, "package")?;
        let version_entries = fs::read_dir(&package_path).map_err(|err| {
            format!(
                "failed to read package directory {}: {err}",
                package_path.display()
            )
        })?;
        for version_entry in version_entries {
            let version_entry = version_entry.map_err(|err| {
                format!(
                    "failed to read version entry under {}: {err}",
                    package_path.display()
                )
            })?;
            let version_path = version_entry.path();
            if !version_entry
                .file_type()
                .map_err(|err| format!("failed to inspect {}: {err}", version_path.display()))?
                .is_dir()
            {
                continue;
            }
            let version = utf8_directory_name(&version_path, "version")?;
            packages.push(InstalledPackage {
                name: name.clone(),
                version,
                package_root: version_path.clone(),
                interfaces_dir: version_path.join("interfaces"),
                targets_dir: version_path.join("targets"),
            });
        }
    }
    packages.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
    Ok(packages)
}

fn utf8_directory_name(path: &Path, kind: &str) -> Result<String, String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("{kind} directory name is not UTF-8: {}", path.display()))
}

/// Resolve `name` or `name@version` in the store.
pub fn find_installed(store_root: &Path, package_id: &str) -> Result<InstalledPackage, String> {
    let (name, version) = split_package_id(package_id);
    let installed = list_installed(store_root)?;
    let matches: Vec<_> = installed
        .into_iter()
        .filter(|pkg| pkg.name == name)
        .collect();
    if matches.is_empty() {
        return Err(format!(
            "package `{name}` is not installed in store {}",
            store_root.display()
        ));
    }
    if let Some(version) = version {
        return matches
            .into_iter()
            .find(|pkg| pkg.version == version)
            .ok_or_else(|| {
                format!(
                    "package `{name}` version `{version}` is not installed in store {}",
                    store_root.display()
                )
            });
    }
    if matches.len() == 1 {
        return Ok(matches.into_iter().next().expect("len checked"));
    }
    let versions = matches
        .iter()
        .map(|pkg| pkg.version.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "package `{name}` has multiple installed versions ({versions}); use `{name}@<version>`"
    ))
}

/// Parse `name` or `name@version`.
pub fn split_package_id(package_id: &str) -> (String, Option<String>) {
    if let Some((name, version)) = package_id.split_once('@') {
        (name.to_owned(), Some(version.to_owned()))
    } else {
        (package_id.to_owned(), None)
    }
}

/// Collect files under an installed package root (relative paths).
pub fn list_package_files(package_root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_files(package_root, package_root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        fs::read_dir(dir).map_err(|err| format!("failed to read {}: {err}", dir.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|err| format!("failed to read entry under {}: {err}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("failed to inspect {}: {err}", path.display()))?;
        if file_type.is_symlink() {
            return Err(format!(
                "installed package contains unsupported symlink {}",
                path.display()
            ));
        }
        if file_type.is_dir() {
            collect_files(root, &path, out)?;
        } else if file_type.is_file() {
            if let Ok(relative) = path.strip_prefix(root) {
                out.push(relative.to_path_buf());
            }
        }
    }
    Ok(())
}

/// Read the first available target-level installed `cista.toml` if present.
pub fn read_any_target_manifest(
    package: &InstalledPackage,
) -> Result<Option<(PathBuf, CistaManifest)>, String> {
    let targets = &package.targets_dir;
    match fs::symlink_metadata(targets) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!(
                "installed package contains unsupported symlink {}",
                targets.display()
            ));
        }
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Ok(None),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("failed to inspect {}: {err}", targets.display())),
    }
    walk_for_manifest(targets)
}

fn walk_for_manifest(dir: &Path) -> Result<Option<(PathBuf, CistaManifest)>, String> {
    let mut entries = fs::read_dir(dir)
        .map_err(|err| format!("failed to read {}: {err}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read entry under {}: {err}", dir.display()))?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("failed to inspect {}: {err}", path.display()))?;
        if file_type.is_symlink() {
            return Err(format!(
                "installed package contains unsupported symlink {}",
                path.display()
            ));
        }
        if file_type.is_dir() {
            if let Some(found) = walk_for_manifest(&path)? {
                return Ok(Some(found));
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some(MANIFEST_FILE) {
            return read_manifest(&path).map(|manifest| Some((path, manifest)));
        }
    }
    Ok(None)
}

/// Resolve a package id or filesystem path for inspect.
pub fn resolve_package_or_path(
    value: &str,
    store: Option<&Path>,
) -> Result<ResolvedInspectTarget, String> {
    let path = Path::new(value);
    if path.exists() {
        let root = normalize_path(path);
        return Ok(ResolvedInspectTarget::Path(root));
    }
    let store_root = store_root(store)?;
    let installed = find_installed(&store_root, value)?;
    Ok(ResolvedInspectTarget::Installed(installed))
}

#[derive(Debug)]
pub enum ResolvedInspectTarget {
    Path(PathBuf),
    Installed(InstalledPackage),
}

#[cfg(test)]
#[path = "store_test.rs"]
mod tests;
