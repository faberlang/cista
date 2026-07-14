use crate::manifest;
use crate::store;
use crate::{credentials, registry_http::RegistryHttpClient};
use std::sync::atomic::{AtomicU64, Ordering};

use super::{env, fs_util, shared, Path, PathBuf};

const REGISTRY_ENV: &str = "CISTA_REGISTRY";
static REMOTE_STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) fn publish_remote(
    package_path: &Path,
    manifest_name: &Path,
    origin: &str,
) -> Result<(), String> {
    let checked = shared::validate_package(package_path, manifest_name, None, false)
        .map_err(|diagnostics| diagnostics.join("; "))?;
    let client = authenticated_client(origin)?;
    let archive = archive_directory(&checked.package_root)?;
    client.publish_package(
        &checked.manifest.source.package,
        &checked.manifest.source.version,
        archive,
    )
}

pub(super) fn fetch_remote_to_cache(
    package_id: &str,
    origin: &str,
    explicit_store: Option<&Path>,
) -> Result<PathBuf, String> {
    let (name, version) = exact_identity(package_id)?;
    let archive = authenticated_client(origin)?.fetch_package(&name, &version)?;
    let destination = store::store_root(explicit_store)?
        .join(".cache")
        .join("registry")
        .join(&name)
        .join(&version);
    install_remote_archive(&archive, &destination, &name, &version)?;
    Ok(destination)
}

fn install_remote_archive(
    archive: &[u8],
    destination: &Path,
    name: &str,
    version: &str,
) -> Result<(), String> {
    let sequence = REMOTE_STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let staging = destination.with_extension(format!("incoming-{}-{sequence}", std::process::id()));
    fs_util::replace_directory(&staging)?;
    let install_result = (|| {
        unpack_archive(archive, &staging)?;
        let manifest_path = staging.join(manifest::MANIFEST_FILE);
        if !manifest_path.is_file() {
            return Err(format!(
                "remote package `{name}@{version}` archive has no {}",
                manifest::MANIFEST_FILE
            ));
        }
        if let Some(meta) = manifest::read_meta_manifest(&manifest_path)? {
            if meta.source.package != name || meta.source.version != version {
                return Err(format!(
                    "remote package `{name}@{version}` archive declares `{}@{}`",
                    meta.source.package, meta.source.version
                ));
            }
            validate_cached_meta_package(&meta)?;
        } else {
            let staged_manifest = manifest::read_manifest(&manifest_path)?;
            if staged_manifest.source.package != name || staged_manifest.source.version != version {
                return Err(format!(
                    "remote package `{name}@{version}` archive declares `{}@{}`",
                    staged_manifest.source.package, staged_manifest.source.version
                ));
            }
            validate_cached_package(&staging)?;
        }
        fs_util::copy_dir_clean(&staging, destination)
    })();
    let cleanup_result = std::fs::remove_dir_all(&staging).map_err(|error| {
        format!(
            "failed to remove remote package staging directory {}: {error}",
            staging.display()
        )
    });
    match (install_result, cleanup_result) {
        (Err(install_error), Err(cleanup_error)) => {
            Err(format!("{install_error}; {cleanup_error}"))
        }
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

pub(super) fn publish(
    package_path: &Path,
    manifest_name: &Path,
    explicit_registry: Option<&Path>,
) -> Result<PathBuf, String> {
    let checked = shared::validate_package(package_path, manifest_name, None, false)
        .map_err(|diagnostics| diagnostics.join("; "))?;
    let registry = resolve_publish_root(&registry_root(explicit_registry)?)?;
    if registry.starts_with(&checked.package_root) {
        return Err(format!(
            "local registry {} cannot be inside published package {}",
            registry.display(),
            checked.package_root.display()
        ));
    }
    std::fs::create_dir_all(&registry).map_err(|error| {
        format!(
            "failed to create local registry {}: {error}",
            registry.display()
        )
    })?;
    let registry = registry.canonicalize().map_err(|error| {
        format!(
            "failed to resolve local registry {}: {error}",
            registry.display()
        )
    })?;
    let destination = registry
        .join(&checked.manifest.source.package)
        .join(&checked.manifest.source.version);
    if destination.starts_with(&checked.package_root) {
        return Err(format!(
            "local registry destination {} cannot be inside published package {}",
            destination.display(),
            checked.package_root.display()
        ));
    }
    verify_registry_publish_path(&registry, &destination)?;
    let package_directory = destination
        .parent()
        .ok_or_else(|| format!("registry package has no parent: {}", destination.display()))?;
    std::fs::create_dir_all(package_directory).map_err(|error| {
        format!(
            "failed to create registry package directory {}: {error}",
            package_directory.display()
        )
    })?;
    fs_util::copy_dir_new(&checked.package_root, &destination).map_err(|error| {
        if error.starts_with("directory already exists:") {
            format!(
                "registry package already exists and is immutable: {}",
                destination.display()
            )
        } else {
            error
        }
    })?;
    Ok(destination)
}

pub(super) fn fetch_to_cache(
    package_id: &str,
    explicit_registry: Option<&Path>,
    explicit_store: Option<&Path>,
) -> Result<PathBuf, String> {
    let (name, version) = exact_identity(package_id)?;
    let registry = registry_root(explicit_registry)?;
    let source = registry.join(&name).join(&version);
    verify_registry_package_path(&registry, &source)?;
    let source_manifest = source.join(manifest::MANIFEST_FILE);
    if !source_manifest.is_file() {
        return Err(format!(
            "package `{name}@{version}` is not published in registry {}",
            registry.display()
        ));
    }
    match manifest::read_meta_manifest(&source_manifest)? {
        Some(meta) => {
            if meta.source.package != name || meta.source.version != version {
                return Err(format!(
                    "registry package `{name}@{version}` declares `{}@{}`",
                    meta.source.package, meta.source.version
                ));
            }
            validate_cached_meta_package(&meta)?;
        }
        None => {
            let manifest = manifest::read_manifest(&source_manifest)?;
            if manifest.source.package != name || manifest.source.version != version {
                return Err(format!(
                    "registry package `{name}@{version}` declares `{}@{}`",
                    manifest.source.package, manifest.source.version
                ));
            }
            validate_cached_package(&source)?;
        }
    }
    let store_root = store::store_root(explicit_store)?;
    let destination = store_root
        .join(".cache")
        .join("registry")
        .join(&name)
        .join(&version);
    fs_util::copy_dir_clean(&source, &destination)?;
    Ok(destination)
}

pub(super) fn reject_meta_install_by_name(
    package_id: &str,
    explicit_registry: Option<&Path>,
) -> Result<(), String> {
    let (name, version) = exact_identity(package_id)?;
    let registry = registry_root(explicit_registry)?;
    let source = registry.join(&name).join(&version);
    verify_registry_package_path(&registry, &source)?;
    let source_manifest = source.join(manifest::MANIFEST_FILE);
    if !source_manifest.is_file() {
        return Err(format!(
            "package `{name}@{version}` is not published in registry {}",
            registry.display()
        ));
    }
    if let Some(meta) = manifest::read_meta_manifest(&source_manifest)? {
        if meta.source.package != name || meta.source.version != version {
            return Err(format!(
                "registry package `{name}@{version}` declares `{}@{}`",
                meta.source.package, meta.source.version
            ));
        }
        validate_cached_meta_package(&meta)?;
        return Err(format!(
            "registry meta package `{name}@{version}` cannot be installed by name yet; install a local pathful meta package with --path or install its dependencies explicitly"
        ));
    }
    Ok(())
}

fn validate_cached_package(package_root: &Path) -> Result<(), String> {
    shared::validate_package(
        package_root,
        Path::new(manifest::MANIFEST_FILE),
        None,
        false,
    )
    .map(|_| ())
    .map_err(|diagnostics| diagnostics.join("; "))
}

fn validate_cached_meta_package(meta: &manifest::MetaManifest) -> Result<(), String> {
    shared::validate_identity(&meta.source.package, &meta.source.version)
        .map_err(|diagnostics| diagnostics.join("; "))?;
    if meta.dependencies.is_empty() {
        return Err("meta package requires at least one [[dependencies]] row".to_owned());
    }
    let mut seen = std::collections::BTreeSet::new();
    for dependency in &meta.dependencies {
        let identity = format!("{}@{}", dependency.package, dependency.version);
        if !seen.insert(identity.clone()) {
            return Err(format!("duplicate meta dependency `{identity}`"));
        }
        shared::validate_identity(&dependency.package, &dependency.version)
            .map_err(|diagnostics| diagnostics.join("; "))?;
        if dependency.path.is_some() {
            return Err(format!(
                "cached meta dependency `{identity}` must not carry a source-relative path"
            ));
        }
    }
    Ok(())
}

fn verify_registry_publish_path(registry: &Path, package: &Path) -> Result<(), String> {
    let existing_parent = package
        .ancestors()
        .find(|ancestor| ancestor.exists())
        .ok_or_else(|| {
            format!(
                "local registry package has no existing parent: {}",
                package.display()
            )
        })?;
    verify_registry_package_path(registry, existing_parent)
}

fn resolve_publish_root(registry: &Path) -> Result<PathBuf, String> {
    let existing_parent = registry
        .ancestors()
        .find(|ancestor| ancestor.exists())
        .ok_or_else(|| {
            format!(
                "local registry has no existing parent: {}",
                registry.display()
            )
        })?;
    let suffix = registry.strip_prefix(existing_parent).map_err(|error| {
        format!(
            "failed to resolve local registry {} from existing parent {}: {error}",
            registry.display(),
            existing_parent.display()
        )
    })?;
    existing_parent
        .canonicalize()
        .map(|parent| parent.join(suffix))
        .map_err(|error| {
            format!(
                "failed to resolve local registry parent {}: {error}",
                existing_parent.display()
            )
        })
}

fn verify_registry_package_path(registry: &Path, package: &Path) -> Result<(), String> {
    let registry = registry.canonicalize().map_err(|error| {
        format!(
            "failed to resolve local registry {}: {error}",
            registry.display()
        )
    })?;
    let package = package.canonicalize().map_err(|error| {
        format!(
            "failed to resolve local registry package {}: {error}",
            package.display()
        )
    })?;
    if !package.starts_with(&registry) {
        return Err(format!(
            "local registry package {} resolves outside registry {}",
            package.display(),
            registry.display()
        ));
    }
    Ok(())
}

fn exact_identity(package_id: &str) -> Result<(String, String), String> {
    let (name, version) = store::split_package_id(package_id);
    let version = version.ok_or_else(|| {
        format!("registry package `{package_id}` must use an exact name@version pin")
    })?;
    if name.is_empty() || version.is_empty() {
        return Err(format!(
            "registry package `{package_id}` must use an exact name@version pin"
        ));
    }
    shared::validate_identity(&name, &version).map_err(|diagnostics| diagnostics.join("; "))?;
    Ok((name, version))
}

fn authenticated_client(origin: &str) -> Result<RegistryHttpClient, String> {
    let path = credentials::default_path()?;
    let token = credentials::token(&path, origin)?
        .ok_or_else(|| format!("no registry credentials stored for {origin}; run `cista login`"))?;
    RegistryHttpClient::new(origin, Some(&token))
}

fn archive_directory(root: &Path) -> Result<Vec<u8>, String> {
    let mut archive = tar::Builder::new(Vec::new());
    for relative in store::list_package_files(root)? {
        archive
            .append_path_with_name(root.join(&relative), &relative)
            .map_err(|error| {
                format!(
                    "failed to archive package file {}: {error}",
                    relative.display()
                )
            })?;
    }
    archive
        .into_inner()
        .map_err(|error| format!("failed to finish package archive: {error}"))
}

fn unpack_archive(bytes: &[u8], destination: &Path) -> Result<(), String> {
    let mut archive = tar::Archive::new(bytes);
    let entries = archive
        .entries()
        .map_err(|error| format!("failed to read remote package archive: {error}"))?;
    for entry in entries {
        let mut entry = entry
            .map_err(|error| format!("failed to read remote package archive entry: {error}"))?;
        let path = entry
            .path()
            .map_err(|error| format!("failed to read remote package archive path: {error}"))?
            .into_owned();
        let entry_type = entry.header().entry_type();
        if !entry_type.is_file() && !entry_type.is_dir() {
            return Err(format!(
                "remote package archive contains unsupported entry {}",
                path.display()
            ));
        }
        if !entry
            .unpack_in(destination)
            .map_err(|error| format!("failed to unpack remote package archive: {error}"))?
        {
            return Err(format!(
                "remote package archive entry escapes destination: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn registry_root(explicit: Option<&Path>) -> Result<PathBuf, String> {
    let path = explicit
        .map(Path::to_path_buf)
        .or_else(|| env::var_os(REGISTRY_ENV).map(PathBuf::from))
        .ok_or_else(|| {
            "local registry is not configured; pass --registry or set CISTA_REGISTRY".to_owned()
        })?;
    Ok(store::normalize_path(&path))
}

#[cfg(test)]
#[path = "registry_test.rs"]
mod tests;
