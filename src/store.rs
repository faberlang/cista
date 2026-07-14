//! Shared package-store layout helpers (cista-owned discovery).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::manifest::{
    read_manifest, read_meta_manifest, CistaManifest, MetaManifest, MANIFEST_FILE,
};

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
        if is_reserved_store_namespace(&name) {
            continue;
        }
        if !is_valid_store_segment(&name, false) {
            continue;
        }
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
            if is_install_transaction_directory(&version) {
                continue;
            }
            if !is_valid_store_segment(&version, true) {
                continue;
            }
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

fn is_reserved_store_namespace(name: &str) -> bool {
    name.starts_with('.')
}

pub(crate) fn validate_store_identity(package: &str, version: &str) -> Result<(), Vec<String>> {
    let mut diagnostics = Vec::new();
    require_non_empty("source.package", package, &mut diagnostics);
    require_non_empty("source.version", version, &mut diagnostics);
    validate_store_segment("source.package", package, false, &mut diagnostics);
    validate_store_segment("source.version", version, true, &mut diagnostics);
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

fn require_non_empty(field: &str, value: &str, diagnostics: &mut Vec<String>) {
    if value.trim().is_empty() {
        diagnostics.push(format!("{field} must not be empty"));
    }
}

fn validate_store_segment(
    field: &str,
    value: &str,
    is_version: bool,
    diagnostics: &mut Vec<String>,
) {
    if value.is_empty() {
        return;
    }
    if !is_valid_store_segment(value, is_version) {
        diagnostics.push(format!(
            "{field} `{value}` is not a valid package store path segment"
        ));
    }
    if is_version && is_install_transaction_directory(value) {
        diagnostics.push(format!(
            "{field} `{value}` collides with Cista install transaction directory namespace"
        ));
    }
}

fn is_valid_store_segment(value: &str, is_version: bool) -> bool {
    !value.contains('/')
        && !value.contains('\\')
        && !value.contains('@')
        && value != "."
        && value != ".."
        && !value.starts_with('.')
        && (!is_version || !is_install_transaction_directory(value))
}

pub(crate) fn is_install_transaction_directory(version: &str) -> bool {
    [".incoming-", ".replaced-"]
        .iter()
        .any(|marker| has_transaction_suffix(version, marker))
}

fn has_transaction_suffix(version: &str, marker: &str) -> bool {
    let Some((base, suffix)) = version.rsplit_once(marker) else {
        return false;
    };
    if base.is_empty() {
        return false;
    }
    let Some((pid, sequence)) = suffix.split_once('-') else {
        return false;
    };
    !pid.is_empty()
        && !sequence.is_empty()
        && pid.bytes().all(|byte| byte.is_ascii_digit())
        && sequence.bytes().all(|byte| byte.is_ascii_digit())
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
    let mut matches: Vec<_> = installed
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
        return matches.pop().ok_or_else(|| {
            format!(
                "package `{name}` is not installed in store {}",
                store_root.display()
            )
        });
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

/// Resolve `name` or `name@version` and validate installed target manifest identity.
pub fn find_verified_installed(
    store_root: &Path,
    package_id: &str,
) -> Result<InstalledPackage, String> {
    let package = find_installed(store_root, package_id)?;
    validate_installed_identity(&package)?;
    Ok(package)
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
    if fs::symlink_metadata(package_root)
        .map_err(|err| format!("failed to inspect {}: {err}", package_root.display()))?
        .file_type()
        .is_symlink()
    {
        return Err(format!(
            "installed package contains unsupported symlink {}",
            package_root.display()
        ));
    }
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
            let relative = path.strip_prefix(root).map_err(|err| {
                format!(
                    "failed to make package file {} relative to {}: {err}",
                    path.display(),
                    root.display()
                )
            })?;
            out.push(relative.to_path_buf());
        } else {
            return Err(format!(
                "installed package contains unsupported entry {}",
                path.display()
            ));
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
    walk_for_manifest(package, targets)
}

fn walk_for_manifest(
    package: &InstalledPackage,
    dir: &Path,
) -> Result<Option<(PathBuf, CistaManifest)>, String> {
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
            if let Some(found) = walk_for_manifest(package, &path)? {
                return Ok(Some(found));
            }
        } else if file_type.is_file()
            && path.file_name().and_then(|n| n.to_str()) == Some(MANIFEST_FILE)
        {
            let manifest = read_manifest(&path)?;
            validate_manifest_identity(package, &path, &manifest)?;
            return Ok(Some((path, manifest)));
        } else if !file_type.is_file() {
            return Err(format!(
                "installed package contains unsupported entry {}",
                path.display()
            ));
        }
    }
    Ok(None)
}

/// Validate all installed target manifests discovered for a package.
pub fn validate_installed_identity(package: &InstalledPackage) -> Result<(), String> {
    validate_root_manifest(package)?;
    validate_target_manifest_tree(package, &package.targets_dir)
}

fn validate_root_manifest(package: &InstalledPackage) -> Result<(), String> {
    let path = package.package_root.join(MANIFEST_FILE);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!(
                "installed package contains unsupported symlink {}",
                path.display()
            ));
        }
        Ok(metadata) if metadata.is_file() => {}
        Ok(metadata) if metadata.is_dir() => {
            return Err(format!(
                "installed package contains unsupported entry {}",
                path.display()
            ));
        }
        Ok(_) => {
            return Err(format!(
                "installed package contains unsupported entry {}",
                path.display()
            ));
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(format!("failed to inspect {}: {err}", path.display())),
    }

    if let Some(meta) = read_meta_manifest(&path)? {
        validate_meta_manifest_identity(package, &path, &meta)?;
        return Ok(());
    }
    let manifest = read_manifest(&path)?;
    validate_manifest_identity(package, &path, &manifest)
}

fn validate_target_manifest_tree(package: &InstalledPackage, dir: &Path) -> Result<(), String> {
    match fs::symlink_metadata(dir) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!(
                "installed package contains unsupported symlink {}",
                dir.display()
            ));
        }
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(format!("failed to inspect {}: {err}", dir.display())),
    }

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
            validate_target_manifest_tree(package, &path)?;
        } else if file_type.is_file()
            && path.file_name().and_then(|name| name.to_str()) == Some(MANIFEST_FILE)
        {
            let manifest = read_manifest(&path)?;
            validate_manifest_identity(package, &path, &manifest)?;
        } else if !file_type.is_file() {
            return Err(format!(
                "installed package contains unsupported entry {}",
                path.display()
            ));
        }
    }
    Ok(())
}

pub fn validate_manifest_identity(
    package: &InstalledPackage,
    manifest_path: &Path,
    manifest: &CistaManifest,
) -> Result<(), String> {
    if manifest.source.package == package.name && manifest.source.version == package.version {
        return Ok(());
    }
    Err(format!(
        "installed package identity mismatch: directory `{}@{}` contains target manifest {} for `{}@{}`",
        package.name,
        package.version,
        manifest_path.display(),
        manifest.source.package,
        manifest.source.version
    ))
}

fn validate_meta_manifest_identity(
    package: &InstalledPackage,
    manifest_path: &Path,
    manifest: &MetaManifest,
) -> Result<(), String> {
    if manifest.source.package == package.name && manifest.source.version == package.version {
        return Ok(());
    }
    Err(format!(
        "installed package identity mismatch: directory `{}@{}` contains root manifest {} for `{}@{}`",
        package.name,
        package.version,
        manifest_path.display(),
        manifest.source.package,
        manifest.source.version
    ))
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
    let installed = find_verified_installed(&store_root, value)?;
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
