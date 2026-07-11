use std::collections::{BTreeMap, BTreeSet};

use crate::manifest::{
    manifest_path, read_manifest, BindingPolicy, CistaManifest, SourceKind, TargetMode,
};

use super::{fs, rust_target, Path, PathBuf};

pub(super) struct CheckedPackage {
    pub package_root: PathBuf,
    pub manifest: CistaManifest,
}

pub(super) fn validate_package(
    package_path: &Path,
    manifest_name: &Path,
    expected_target_language: Option<&str>,
    verify_build: bool,
) -> Result<CheckedPackage, Vec<String>> {
    let package_root = normalize_path(package_path);
    let manifest_path = manifest_path(&package_root, Some(manifest_name));
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

    let interface_root = package_root.join(&manifest.source.interfaces);
    let interface_symbols = validate_interfaces(&interface_root, &manifest, &mut diagnostics);
    validate_target_paths(&package_root, &manifest, &mut diagnostics);
    validate_bindings(&manifest, &interface_symbols, &mut diagnostics);

    if verify_build {
        rust_target::verify_target_build(&package_root, &manifest, &mut diagnostics);
    }

    if !diagnostics.is_empty() {
        Err(diagnostics)
    } else {
        Ok(CheckedPackage {
            package_root,
            manifest,
        })
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
    package_root: &Path,
    manifest: &CistaManifest,
    diagnostics: &mut Vec<String>,
) {
    if let Some(source) = &manifest.source.sources {
        let path = package_root.join(source);
        if !path.exists() {
            diagnostics.push(format!(
                "source.sources path does not exist: {}",
                path.display()
            ));
        }
    }

    if let Some(source) = &manifest.target.source {
        let path = package_root.join(source);
        if !path.is_dir() {
            diagnostics.push(format!(
                "target.source does not point to a directory: {}",
                path.display()
            ));
        }
        if manifest.target.language == rust_target::RUST_LANGUAGE {
            let cargo_toml = path.join("Cargo.toml");
            if !cargo_toml.is_file() {
                diagnostics.push(format!(
                    "rust target.source is missing Cargo.toml: {}",
                    cargo_toml.display()
                ));
            }
        }
    }

    if let Some(artifact) = &manifest.target.artifact {
        let path = package_root.join(artifact);
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
    if module
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
