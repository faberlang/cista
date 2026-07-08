use crate::cli::InstallArgs;
use crate::manifest::{
    CistaManifest, SourceKind, SourceSection, TargetFlags, TargetMode, TargetSection,
};

use super::{fs, fs_util, rust_target, shared, CommandResult, Path, PathBuf};

pub fn run(args: InstallArgs) -> CommandResult {
    let checked = shared::validate_package(
        &args.path,
        &args.manifest,
        Some(&args.target_language),
        args.verify_target_build,
    )?;
    let store_root = shared::resolve_store_root(args.store.as_deref()).map_err(|err| vec![err])?;
    install_checked_package(&checked, &store_root)?;
    Ok(())
}

fn install_checked_package(checked: &shared::CheckedPackage, store_root: &Path) -> CommandResult {
    let manifest = &checked.manifest;
    ensure_rust_source_install(manifest)?;

    let target_triple = rust_target::rust_host_triple().map_err(|err| vec![err])?;
    let rustc_version = rust_target::rustc_version().map_err(|err| vec![err])?;
    let artifact = rust_target::build_rust_library(&checked.package_root, manifest)
        .map_err(|err| vec![err])?;

    let package_store_root = shared::package_store_root(store_root, manifest);
    install_interfaces(&checked.package_root, manifest, &package_store_root)
        .map_err(|err| vec![err])?;
    install_built_rust_target(
        manifest,
        &artifact,
        &package_store_root,
        &target_triple,
        &rustc_version,
    )
    .map_err(|err| vec![err])?;

    println!(
        "installed: {} {} -> {}",
        manifest.source.package,
        manifest.source.version,
        package_store_root.display()
    );
    Ok(())
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

fn install_built_rust_target(
    manifest: &CistaManifest,
    artifact: &Path,
    package_store_root: &Path,
    target_triple: &str,
    rustc_version: &str,
) -> Result<(), String> {
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
    )
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
            interfaces: PathBuf::from("../../../interfaces"),
            sources: None,
        },
        target: TargetSection {
            language: source_manifest.target.language.clone(),
            mode: TargetMode::Artifact,
            binding_policy: source_manifest.target.binding_policy.clone(),
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
