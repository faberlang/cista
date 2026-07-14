use std::collections::{BTreeMap, BTreeSet};
use std::path::Component;

use crate::manifest::{read_manifest, BindingPolicy, CistaManifest, SourceKind, TargetMode};
use fs2::FileExt;

use super::{fs, rust_target, Path, PathBuf};

pub(super) const STORE_MUTATION_LOCK_FILE: &str = ".cista-install.lock";

pub(super) struct StoreMutationLocks {
    _files: Vec<fs::File>,
}

pub(super) fn acquire_store_mutation_locks(
    store_root: &Path,
    project_root: Option<&Path>,
) -> Result<StoreMutationLocks, String> {
    let mut paths = vec![store_root.join(STORE_MUTATION_LOCK_FILE)];
    if let Some(project_root) = project_root {
        paths.push(project_root.join(STORE_MUTATION_LOCK_FILE));
    }
    paths.sort();
    paths.dedup();

    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "failed to create store mutation lock directory {}: {error}",
                    parent.display()
                )
            })?;
        }
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| {
                format!(
                    "failed to open store mutation lock {}: {error}",
                    path.display()
                )
            })?;
        file.lock_exclusive().map_err(|error| {
            format!(
                "failed to acquire store mutation lock {}: {error}",
                path.display()
            )
        })?;
        files.push(file);
    }
    Ok(StoreMutationLocks { _files: files })
}

pub(super) struct CheckedPackage {
    pub package_root: PathBuf,
    pub manifest: CistaManifest,
    pub paths: PackagePaths,
}

pub(super) struct PackagePaths {
    pub interfaces: Option<PathBuf>,
    pub target_source: Option<PathBuf>,
    pub artifact: Option<PathBuf>,
}

pub(super) fn validate_package(
    package_path: &Path,
    manifest_name: &Path,
    expected_target_language: Option<&str>,
    verify_build: bool,
) -> Result<CheckedPackage, Vec<String>> {
    let package_root = normalize_path(package_path);
    let manifest_path =
        package_manifest_path(&package_root, manifest_name).map_err(|error| vec![error])?;
    let manifest = read_manifest(&manifest_path).map_err(|err| vec![err])?;

    let mut diagnostics = Vec::new();
    validate_manifest_shape(&manifest, &mut diagnostics);

    if let Some(expected) = expected_target_language {
        if manifest.target.language != expected {
            diagnostics.push(format!(
                "target language mismatch: expected `{expected}`, manifest declares `{}`",
                manifest.target.language
            ));
        }
    }

    let paths = resolve_package_paths(&package_root, &manifest, &mut diagnostics);
    let interface_symbols = paths
        .interfaces
        .as_deref()
        .map(|interface_root| {
            validate_interfaces(&package_root, interface_root, &manifest, &mut diagnostics)
        })
        .unwrap_or_default();
    validate_target_paths(&paths, &manifest, &mut diagnostics);
    validate_bindings(&manifest, &interface_symbols, &mut diagnostics);

    if verify_build {
        rust_target::verify_target_build(
            &manifest,
            paths.target_source.as_deref(),
            &mut diagnostics,
        );
    }

    if !diagnostics.is_empty() {
        Err(diagnostics)
    } else {
        Ok(CheckedPackage {
            package_root,
            manifest,
            paths,
        })
    }
}

pub(super) fn package_manifest_path(
    package_root: &Path,
    manifest_name: &Path,
) -> Result<PathBuf, String> {
    resolve_package_path(package_root, "manifest", manifest_name)
}

/// Resolve a local meta dependency within the package collection containing
/// the meta package. Sibling paths such as `../true` are valid; traversal
/// beyond that collection and symlink escapes are not.
pub(super) fn resolve_meta_dependency_path(
    meta_root: &Path,
    field: &str,
    path: &Path,
) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() {
        return Err(format!("{field} path must not be empty"));
    }
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::Prefix(_) | Component::RootDir))
    {
        return Err(format!("{field} path must be relative: {}", path.display()));
    }

    let meta_root = meta_root.canonicalize().map_err(|error| {
        format!(
            "{field} package root cannot be resolved: {}: {error}",
            meta_root.display()
        )
    })?;
    let package_collection_root = meta_root.parent().ok_or_else(|| {
        format!(
            "{field} package root has no containing collection: {}",
            meta_root.display()
        )
    })?;
    let candidate = meta_root.join(path);
    let resolved = candidate.canonicalize().map_err(|error| {
        format!(
            "{field} path cannot be resolved: {}: {error}",
            candidate.display()
        )
    })?;
    if !resolved.starts_with(package_collection_root) {
        return Err(format!(
            "{field} path resolves outside package collection root: {}",
            resolved.display()
        ));
    }
    Ok(resolved)
}

pub(super) fn resolve_package_path(
    package_root: &Path,
    field: &str,
    path: &Path,
) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() {
        return Err(format!("{field} path must not be empty"));
    }
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::Prefix(_) | Component::RootDir))
    {
        return Err(format!("{field} path must be relative: {}", path.display()));
    }
    if path
        .components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(format!(
            "{field} path must be normalized without `.` or `..` segments: {}",
            path.display()
        ));
    }

    let candidate = package_root.join(path);
    let resolved = candidate.canonicalize().map_err(|error| {
        format!(
            "{field} path cannot be resolved: {}: {error}",
            candidate.display()
        )
    })?;
    if !resolved.starts_with(package_root) {
        return Err(format!(
            "{field} path resolves outside package root: {}",
            resolved.display()
        ));
    }
    Ok(resolved)
}

fn resolve_package_paths(
    package_root: &Path,
    manifest: &CistaManifest,
    diagnostics: &mut Vec<String>,
) -> PackagePaths {
    let interfaces = resolve_manifest_path(
        package_root,
        "source.interfaces",
        &manifest.source.interfaces,
        diagnostics,
    );
    if let Some(path) = manifest.source.sources.as_deref() {
        validate_manifest_path(package_root, "source.sources", path, diagnostics);
    }
    let target_source =
        manifest.target.source.as_deref().and_then(|path| {
            resolve_manifest_path(package_root, "target.source", path, diagnostics)
        });
    let artifact =
        manifest.target.artifact.as_deref().and_then(|path| {
            resolve_manifest_path(package_root, "target.artifact", path, diagnostics)
        });
    PackagePaths {
        interfaces,
        target_source,
        artifact,
    }
}

fn resolve_manifest_path(
    package_root: &Path,
    field: &str,
    path: &Path,
    diagnostics: &mut Vec<String>,
) -> Option<PathBuf> {
    match resolve_package_path(package_root, field, path) {
        Ok(path) => Some(path),
        Err(error) => {
            diagnostics.push(error);
            None
        }
    }
}

fn validate_manifest_path(
    package_root: &Path,
    field: &str,
    path: &Path,
    diagnostics: &mut Vec<String>,
) {
    if let Err(error) = resolve_package_path(package_root, field, path) {
        diagnostics.push(error);
    }
}

pub(super) fn resolve_store_root(explicit_store: Option<&Path>) -> Result<PathBuf, String> {
    crate::store::store_root(explicit_store)
}

pub(super) fn package_store_root(store_root: &Path, manifest: &CistaManifest) -> PathBuf {
    store_root
        .join(&manifest.source.package)
        .join(&manifest.source.version)
}

pub(super) fn normalize_path(path: &Path) -> PathBuf {
    crate::store::normalize_path(path)
}

fn validate_manifest_shape(manifest: &CistaManifest, diagnostics: &mut Vec<String>) {
    require_non_empty("source.package", &manifest.source.package, diagnostics);
    require_non_empty("source.version", &manifest.source.version, diagnostics);
    require_non_empty("source.faber_min", &manifest.source.faber_min, diagnostics);
    require_non_empty("target.language", &manifest.target.language, diagnostics);
    validate_store_segment("source.package", &manifest.source.package, diagnostics);
    validate_store_segment("source.version", &manifest.source.version, diagnostics);

    let source_kind_matches_target_mode = matches!(
        (manifest.source.kind, manifest.target.mode),
        (SourceKind::Source, TargetMode::Compile) | (SourceKind::Artifact, TargetMode::Artifact)
    );
    if !source_kind_matches_target_mode {
        diagnostics.push(format!(
            "source.kind `{}` is incompatible with target.mode `{}`",
            manifest.source.kind.kebab_name(),
            manifest.target.mode.kebab_name()
        ));
    }
    if matches!(manifest.source.kind, SourceKind::Artifact) && manifest.source.sources.is_some() {
        diagnostics.push("source kind `artifact` forbids source.sources".to_owned());
    }

    match manifest.target.mode {
        TargetMode::Compile => {
            if manifest.target.artifact.is_some() {
                diagnostics.push("target mode `compile` forbids target.artifact".to_owned());
            }
            if manifest.target.triple.is_some() {
                diagnostics.push("target mode `compile` forbids target.triple".to_owned());
            }
            if manifest.target.rustc.is_some() {
                diagnostics.push("target mode `compile` forbids target.rustc".to_owned());
            }
            // Pure Faber packages (`binding_policy = generated`) may ship
            // interfaces only — no native target.source / [target.compile].
            // Hand-written native targets still require both fields.
            let interfaces_only =
                matches!(manifest.target.binding_policy, BindingPolicy::Generated)
                    && manifest.target.source.is_none();
            if !interfaces_only {
                if manifest.target.source.is_none() {
                    diagnostics.push("target mode `compile` requires target.source".to_owned());
                }
                if manifest.target.compile.is_none() {
                    diagnostics.push("target mode `compile` requires [target.compile]".to_owned());
                }
            }
        }
        TargetMode::Artifact => {
            if manifest.target.source.is_some() {
                diagnostics.push("target mode `artifact` forbids target.source".to_owned());
            }
            if manifest.target.compile.is_some() {
                diagnostics.push("target mode `artifact` forbids [target.compile]".to_owned());
            }
            if manifest.target.artifact.is_none() {
                diagnostics.push("target mode `artifact` requires target.artifact".to_owned());
            }
            if manifest.target.triple.is_none() {
                diagnostics.push("target mode `artifact` requires target.triple".to_owned());
            }
            if manifest.target.rustc.is_none() {
                diagnostics.push("target mode `artifact` requires target.rustc".to_owned());
            }
        }
    }

    match manifest.target.binding_policy {
        BindingPolicy::Generated if !manifest.bindings.is_empty() => {
            diagnostics.push("binding policy `generated` forbids [[bindings]] rows".to_owned());
        }
        BindingPolicy::Manifest if manifest.bindings.is_empty() => {
            diagnostics.push(
                "binding policy `manifest` requires at least one [[bindings]] row".to_owned(),
            );
        }
        BindingPolicy::Generated | BindingPolicy::Manifest => {}
    }

    for (index, binding) in manifest.bindings.iter().enumerate() {
        let prefix = format!("bindings[{index}]");
        require_non_empty(
            &format!("{prefix}.source_module"),
            &binding.source_module,
            diagnostics,
        );
        require_non_empty(
            &format!("{prefix}.source_symbol"),
            &binding.source_symbol,
            diagnostics,
        );
        require_non_empty(&format!("{prefix}.target"), &binding.target, diagnostics);
        validate_module_path(&binding.source_module, diagnostics);
    }
}

fn validate_interfaces(
    package_root: &Path,
    interface_root: &Path,
    manifest: &CistaManifest,
    diagnostics: &mut Vec<String>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut symbols = BTreeMap::new();
    if !interface_root.is_dir() {
        diagnostics.push(format!(
            "source.interfaces does not point to a directory: {}",
            interface_root.display()
        ));
        return symbols;
    }

    for binding in &manifest.bindings {
        if symbols.contains_key(&binding.source_module) {
            continue;
        }
        let interface_path = interface_root.join(format!("{}.fab", binding.source_module));
        let interface_path = match interface_path.canonicalize() {
            Ok(path) if path.starts_with(package_root) => path,
            Ok(path) => {
                diagnostics.push(format!(
                    "interface path resolves outside package root: {}",
                    path.display()
                ));
                continue;
            }
            Err(err) => {
                diagnostics.push(format!(
                    "failed to resolve interface {}: {err}",
                    interface_path.display()
                ));
                continue;
            }
        };
        match read_interface_symbols(&interface_path) {
            Ok(found) => {
                symbols.insert(binding.source_module.clone(), found);
            }
            Err(err) => diagnostics.push(err),
        }
    }

    symbols
}

fn validate_target_paths(
    paths: &PackagePaths,
    manifest: &CistaManifest,
    diagnostics: &mut Vec<String>,
) {
    if let Some(path) = &paths.target_source {
        if !path.is_dir() {
            diagnostics.push(format!(
                "target.source does not point to a directory: {}",
                path.display()
            ));
        }
        if manifest.target.language == rust_target::RUST_LANGUAGE {
            let cargo_toml = path.join("Cargo.toml");
            match cargo_toml.canonicalize() {
                Ok(resolved) if resolved.starts_with(path) => {}
                Ok(resolved) => diagnostics.push(format!(
                    "rust target Cargo.toml resolves outside target.source: {}",
                    resolved.display()
                )),
                Err(_) => diagnostics.push(format!(
                    "rust target.source is missing Cargo.toml: {}",
                    cargo_toml.display()
                )),
            }
        }
    }

    if let Some(path) = &paths.artifact {
        if !path.is_file() {
            diagnostics.push(format!(
                "target.artifact does not point to a file: {}",
                path.display()
            ));
        }
    }
}

fn validate_bindings(
    manifest: &CistaManifest,
    interface_symbols: &BTreeMap<String, BTreeSet<String>>,
    diagnostics: &mut Vec<String>,
) {
    for binding in &manifest.bindings {
        let Some(symbols) = interface_symbols.get(&binding.source_module) else {
            continue;
        };
        if !symbols.contains(&binding.source_symbol) {
            diagnostics.push(format!(
                "binding source symbol `{}` not found in module `{}`",
                binding.source_symbol, binding.source_module
            ));
        }
    }
}

fn read_interface_symbols(interface_path: &Path) -> Result<BTreeSet<String>, String> {
    let source = fs::read_to_string(interface_path).map_err(|err| {
        format!(
            "failed to read interface {}: {err}",
            interface_path.display()
        )
    })?;
    let mut symbols = BTreeSet::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        let Some(after_keyword) = trimmed.strip_prefix("functio ") else {
            continue;
        };
        let Some((name, _rest)) = after_keyword.split_once('(') else {
            continue;
        };
        let name = name.trim();
        if !name.is_empty() {
            symbols.insert(name.to_owned());
        }
    }
    Ok(symbols)
}

fn validate_module_path(module: &str, diagnostics: &mut Vec<String>) {
    if module.is_empty() {
        return;
    }
    if module.contains('\\')
        || module
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        diagnostics.push(format!(
            "invalid source_module `{module}`: module paths must not contain empty, dot, or dot-dot segments"
        ));
    }
}

fn validate_store_segment(field: &str, value: &str, diagnostics: &mut Vec<String>) {
    if value.is_empty() {
        return;
    }
    if value.contains('/')
        || value.contains('\\')
        || value == "."
        || value == ".."
        || value.starts_with('.')
    {
        diagnostics.push(format!(
            "{field} `{value}` is not a valid package store path segment"
        ));
    }
}

pub(super) fn validate_identity(package: &str, version: &str) -> Result<(), Vec<String>> {
    let mut diagnostics = Vec::new();
    require_non_empty("source.package", package, &mut diagnostics);
    require_non_empty("source.version", version, &mut diagnostics);
    validate_store_segment("source.package", package, &mut diagnostics);
    validate_store_segment("source.version", version, &mut diagnostics);
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

#[cfg(test)]
#[path = "shared_test.rs"]
mod tests;
