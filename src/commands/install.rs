use crate::cli::InstallArgs;
use crate::faber_lock::{self, locked_from_install, InstalledLockInput};
use crate::manifest::{
    self, BindingPolicy, CistaManifest, MetaManifest, PackageRole, SourceKind, SourceSection,
    TargetFlags, TargetMode, TargetSection,
};
use crate::project_manifest::{self, PROJECT_MANIFEST};

use super::{env, fs, fs_util, registry, rust_target, shared, CommandResult, Path, PathBuf};

/// Packages that are platform defaults: lock rewrite does not require a
/// matching `faber.toml` `[dependencies]` entry.
const PLATFORM_DEFAULT_PACKAGES: &[&str] = &["norma"];

pub fn run(args: InstallArgs) -> CommandResult {
    let package_path = match (&args.path, &args.package) {
        (Some(path), None) => path.clone(),
        (None, Some(package)) => {
            registry::fetch_to_cache(package, args.registry.as_deref(), args.store.as_deref())
                .map_err(|err| vec![err])?
        }
        (Some(_), Some(_)) => {
            return Err(vec![
                "install accepts either --path or name@version, not both".to_owned(),
            ])
        }
        (None, None) => return Err(vec!["install requires --path or name@version".to_owned()]),
    };
    let package_root = shared::normalize_path(&package_path);
    let manifest_path = manifest::manifest_path(&package_root, Some(&args.manifest));
    if let Some(meta) = manifest::read_meta_manifest(&manifest_path).map_err(|err| vec![err])? {
        return install_meta_package(&args, &package_root, &meta);
    }
    let checked = shared::validate_package(
        &package_path,
        &args.manifest,
        Some(&args.target_language),
        args.verify_target_build,
    )?;
    let store_root = shared::resolve_store_root(args.store.as_deref()).map_err(|err| vec![err])?;
    let installed = install_checked_package(&checked, &store_root)?;

    if let Some(project_root) = resolve_project_root(args.project.as_deref())? {
        rewrite_project_lock(&project_root, &checked, &installed)?;
    }

    Ok(())
}

fn install_meta_package(
    args: &InstallArgs,
    package_root: &Path,
    meta: &MetaManifest,
) -> CommandResult {
    if !matches!(meta.source.role, PackageRole::Meta) {
        return Err(vec![
            "meta manifest requires source.role = `meta`".to_owned()
        ]);
    }
    shared::validate_identity(&meta.source.package, &meta.source.version)?;
    if meta.dependencies.is_empty() {
        return Err(vec![
            "meta package requires at least one [[dependencies]] row".to_owned(),
        ]);
    }
    if args.project.is_some() {
        return Err(vec![
            "meta package install does not rewrite a project lock; omit --project".to_owned(),
        ]);
    }
    let store_root = shared::resolve_store_root(args.store.as_deref()).map_err(|err| vec![err])?;
    let mut seen = std::collections::BTreeSet::new();
    let mut checked_dependencies = Vec::with_capacity(meta.dependencies.len());
    for dependency in &meta.dependencies {
        let identity = format!("{}@{}", dependency.package, dependency.version);
        if !seen.insert(identity.clone()) {
            return Err(vec![format!("duplicate meta dependency `{identity}`")]);
        }
        let dependency_path = dependency.path.as_deref().ok_or_else(|| {
            vec![format!(
                "local meta dependency `{identity}` requires a relative path"
            )]
        })?;
        let dependency_root = package_root.join(dependency_path);
        let checked = shared::validate_package(
            &dependency_root,
            &args.manifest,
            Some(&args.target_language),
            args.verify_target_build,
        )?;
        if checked.manifest.source.package != dependency.package
            || checked.manifest.source.version != dependency.version
        {
            return Err(vec![format!(
                "meta dependency `{identity}` resolves to `{}@{}` at {}",
                checked.manifest.source.package,
                checked.manifest.source.version,
                dependency_root.display()
            )]);
        }
        checked_dependencies.push(checked);
    }
    for checked in &checked_dependencies {
        install_checked_package(checked, &store_root)?;
    }

    let meta_root = store_root
        .join(&meta.source.package)
        .join(&meta.source.version);
    fs_util::replace_directory(&meta_root).map_err(|err| vec![err])?;
    let installed_meta = MetaManifest {
        source: meta.source.clone(),
        dependencies: meta
            .dependencies
            .iter()
            .cloned()
            .map(|mut dependency| {
                dependency.path = None;
                dependency
            })
            .collect(),
    };
    let contents = toml::to_string_pretty(&installed_meta)
        .map_err(|err| vec![format!("failed to render installed meta manifest: {err}")])?;
    fs::write(meta_root.join(manifest::MANIFEST_FILE), contents)
        .map_err(|err| vec![format!("failed to write installed meta manifest: {err}")])?;
    println!(
        "installed: {} {} -> {} (meta, {} dependencies)",
        meta.source.package,
        meta.source.version,
        meta_root.display(),
        meta.dependencies.len()
    );
    Ok(())
}

struct InstalledPaths {
    package_store_root: PathBuf,
    target_triple: String,
    rustc_version: String,
    /// Relative artifact file name when an rlib was installed; empty for
    /// interfaces-only packages.
    artifact_name: PathBuf,
    interfaces_only: bool,
}

fn install_checked_package(
    checked: &shared::CheckedPackage,
    store_root: &Path,
) -> Result<InstalledPaths, Vec<String>> {
    let manifest = &checked.manifest;
    ensure_rust_source_install(manifest)?;

    let package_store_root = shared::package_store_root(store_root, manifest);
    install_interfaces(&checked.package_root, manifest, &package_store_root)
        .map_err(|err| vec![err])?;

    let interfaces_only = is_interfaces_only_package(manifest);
    let (target_triple, rustc_version, artifact_name) = if interfaces_only {
        install_interfaces_only_target(manifest, &package_store_root).map_err(|err| vec![err])?
    } else {
        let target_triple = rust_target::rust_host_triple().map_err(|err| vec![err])?;
        let rustc_version = rust_target::rustc_version().map_err(|err| vec![err])?;
        let artifact = rust_target::build_rust_artifact(&checked.package_root, manifest)
            .map_err(|err| vec![err])?;
        let artifact_name = install_built_rust_target(
            manifest,
            &artifact,
            &package_store_root,
            &target_triple,
            &rustc_version,
        )
        .map_err(|err| vec![err])?;
        (target_triple, rustc_version, artifact_name)
    };

    println!(
        "installed: {} {} -> {}{}",
        manifest.source.package,
        manifest.source.version,
        package_store_root.display(),
        if interfaces_only {
            " (interfaces only)"
        } else {
            ""
        }
    );

    Ok(InstalledPaths {
        package_store_root,
        target_triple,
        rustc_version,
        artifact_name,
        interfaces_only,
    })
}

fn is_interfaces_only_package(manifest: &CistaManifest) -> bool {
    matches!(manifest.target.binding_policy, BindingPolicy::Generated)
        && matches!(manifest.target.mode, TargetMode::Compile)
        && manifest.target.source.is_none()
}

fn ensure_rust_source_install(manifest: &CistaManifest) -> CommandResult {
    if manifest.target.language != rust_target::RUST_LANGUAGE {
        return Err(vec![format!(
            "install --path currently supports target.language = `{}`; got `{}`",
            rust_target::RUST_LANGUAGE,
            manifest.target.language
        )]);
    }
    if !matches!(manifest.source.kind, SourceKind::Source) {
        return Err(vec![
            "install --path currently requires source.kind = `source`".to_owned(),
        ]);
    }
    if !matches!(manifest.target.mode, TargetMode::Compile) {
        return Err(vec![
            "install --path currently requires target.mode = `compile`".to_owned(),
        ]);
    }
    Ok(())
}

fn install_interfaces(
    package_root: &Path,
    manifest: &CistaManifest,
    package_store_root: &Path,
) -> Result<(), String> {
    let interface_source = package_root.join(&manifest.source.interfaces);
    let interface_destination = package_store_root.join("interfaces");
    fs_util::copy_dir_clean(&interface_source, &interface_destination)
}

/// Snapshot a pure-Faber package: interfaces already copied; write thin target metadata.
fn install_interfaces_only_target(
    manifest: &CistaManifest,
    package_store_root: &Path,
) -> Result<(String, String, PathBuf), String> {
    let target_triple = rust_target::rust_host_triple().unwrap_or_else(|_| "unknown".to_owned());
    let rustc_version = rust_target::rustc_version().unwrap_or_else(|_| "unknown".to_owned());
    let target_destination = package_store_root
        .join("targets")
        .join(&manifest.target.language)
        .join(&target_triple);
    fs_util::replace_directory(&target_destination)?;

    let installed = CistaManifest {
        source: SourceSection {
            package: manifest.source.package.clone(),
            version: manifest.source.version.clone(),
            faber_min: manifest.source.faber_min.clone(),
            kind: SourceKind::Source,
            role: manifest.source.role,
            interfaces: PathBuf::from("../../../interfaces"),
            sources: None,
        },
        target: TargetSection {
            language: manifest.target.language.clone(),
            mode: TargetMode::Compile,
            binding_policy: BindingPolicy::Generated,
            source: None,
            artifact: None,
            crate_name: manifest.target.crate_name.clone(),
            triple: Some(target_triple.clone()),
            rustc: Some(rustc_version.clone()),
            flags: manifest.target.flags.clone(),
            compile: manifest.target.compile.clone(),
        },
        bindings: Vec::new(),
    };
    let manifest_contents = toml::to_string_pretty(&installed)
        .map_err(|err| format!("failed to render installed cista.toml: {err}"))?;
    let installed_manifest_path = target_destination.join("cista.toml");
    fs::write(&installed_manifest_path, manifest_contents).map_err(|err| {
        format!(
            "failed to write installed manifest {}: {err}",
            installed_manifest_path.display()
        )
    })?;
    Ok((target_triple, rustc_version, PathBuf::new()))
}

fn install_built_rust_target(
    manifest: &CistaManifest,
    artifact: &Path,
    package_store_root: &Path,
    target_triple: &str,
    rustc_version: &str,
) -> Result<PathBuf, String> {
    let target_destination = package_store_root
        .join("targets")
        .join(&manifest.target.language)
        .join(target_triple);
    fs_util::replace_directory(&target_destination)?;

    let Some(artifact_name) = artifact.file_name() else {
        return Err(format!(
            "built artifact path has no file name: {}",
            artifact.display()
        ));
    };
    let artifact_destination = target_destination.join(artifact_name);
    fs::copy(artifact, &artifact_destination).map_err(|err| {
        format!(
            "failed to install artifact {} to {}: {err}",
            artifact.display(),
            artifact_destination.display()
        )
    })?;

    write_installed_target_manifest(
        manifest,
        &target_destination,
        Path::new(artifact_name),
        target_triple,
        rustc_version,
    )?;
    Ok(PathBuf::from(artifact_name))
}

fn write_installed_target_manifest(
    manifest: &CistaManifest,
    target_destination: &Path,
    artifact_name: &Path,
    target_triple: &str,
    rustc_version: &str,
) -> Result<(), String> {
    let installed_manifest =
        artifact_manifest(manifest, artifact_name, target_triple, rustc_version);
    let manifest_contents = toml::to_string_pretty(&installed_manifest)
        .map_err(|err| format!("failed to render installed cista.toml: {err}"))?;
    let installed_manifest_path = target_destination.join("cista.toml");
    fs::write(&installed_manifest_path, manifest_contents).map_err(|err| {
        format!(
            "failed to write installed manifest {}: {err}",
            installed_manifest_path.display()
        )
    })
}

fn artifact_manifest(
    source_manifest: &CistaManifest,
    artifact_name: &Path,
    target_triple: &str,
    rustc_version: &str,
) -> CistaManifest {
    CistaManifest {
        source: SourceSection {
            package: source_manifest.source.package.clone(),
            version: source_manifest.source.version.clone(),
            faber_min: source_manifest.source.faber_min.clone(),
            kind: SourceKind::Artifact,
            role: source_manifest.source.role,
            interfaces: PathBuf::from("../../../interfaces"),
            sources: None,
        },
        target: TargetSection {
            language: source_manifest.target.language.clone(),
            mode: TargetMode::Artifact,
            binding_policy: source_manifest.target.binding_policy,
            source: None,
            artifact: Some(PathBuf::from(artifact_name)),
            crate_name: source_manifest.target.crate_name.clone(),
            triple: Some(target_triple.to_owned()),
            rustc: Some(rustc_version.to_owned()),
            flags: source_manifest
                .target
                .flags
                .clone()
                .or_else(|| compile_flags(source_manifest)),
            compile: None,
        },
        bindings: source_manifest.bindings.clone(),
    }
}

fn compile_flags(manifest: &CistaManifest) -> Option<TargetFlags> {
    manifest.target.compile.as_ref().map(|compile| TargetFlags {
        edition: Some(compile.edition.clone()),
    })
}

/// Prefer `--project`; else cwd when it contains faber.toml; else None (store-only).
fn resolve_project_root(explicit: Option<&Path>) -> Result<Option<PathBuf>, Vec<String>> {
    if let Some(path) = explicit {
        let root = shared::normalize_path(path);
        let manifest = root.join(PROJECT_MANIFEST);
        if !manifest.is_file() {
            return Err(vec![format!(
                "project root missing {}: {}",
                PROJECT_MANIFEST,
                root.display()
            )]);
        }
        return Ok(Some(root));
    }

    let cwd = env::current_dir().map_err(|err| vec![format!("failed to read cwd: {err}")])?;
    let manifest = cwd.join(PROJECT_MANIFEST);
    if manifest.is_file() {
        Ok(Some(shared::normalize_path(&cwd)))
    } else {
        Ok(None)
    }
}

fn rewrite_project_lock(
    project_root: &Path,
    checked: &shared::CheckedPackage,
    installed: &InstalledPaths,
) -> CommandResult {
    let project_manifest_path = project_root.join(PROJECT_MANIFEST);
    let project_manifest =
        project_manifest::read_project_manifest(&project_manifest_path).map_err(|err| vec![err])?;
    let package = &checked.manifest.source.package;
    let version = &checked.manifest.source.version;

    let is_platform_default = PLATFORM_DEFAULT_PACKAGES.contains(&package.as_str());
    if !is_platform_default {
        project_manifest::require_exact_dependency(&project_manifest, package, version)
            .map_err(|err| vec![err])?;
    }

    let crate_name = checked
        .manifest
        .target
        .crate_name
        .as_deref()
        .unwrap_or(package);
    let record = locked_from_install(InstalledLockInput {
        name: package,
        version,
        source_path: &checked.package_root,
        package_store_root: &installed.package_store_root,
        target_language: &checked.manifest.target.language,
        target_triple: &installed.target_triple,
        artifact_name: installed.artifact_name.as_path(),
        crate_name,
        rustc: &installed.rustc_version,
        kind: "source",
        has_artifact: !installed.interfaces_only && !installed.artifact_name.as_os_str().is_empty(),
    });

    let lock_path = faber_lock::lock_path(project_root);
    let mut lock = faber_lock::read_lock(&lock_path).map_err(|err| vec![err])?;
    faber_lock::upsert_package(&mut lock, record);
    faber_lock::write_lock(&lock_path, &lock).map_err(|err| vec![err])?;
    println!(
        "updated lock: {}{}",
        lock_path.display(),
        if is_platform_default {
            " (platform default)"
        } else {
            ""
        }
    );
    Ok(())
}

#[cfg(test)]
#[path = "install_test.rs"]
mod tests;
