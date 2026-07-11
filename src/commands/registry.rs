use crate::manifest;
use crate::store;
use crate::{credentials, registry_http::RegistryHttpClient};

use super::{env, fs_util, shared, Path, PathBuf};

const REGISTRY_ENV: &str = "CISTA_REGISTRY";

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
    let staging = destination.with_extension(format!("incoming-{}", std::process::id()));
    fs_util::replace_directory(&staging)?;
    unpack_archive(&archive, &staging)?;
    if !staging.join(manifest::MANIFEST_FILE).is_file() {
        return Err(format!(
            "remote package `{name}@{version}` archive has no {}",
            manifest::MANIFEST_FILE
        ));
    }
    if destination.exists() {
        std::fs::remove_dir_all(&destination).map_err(|error| {
            format!(
                "failed to replace remote package cache {}: {error}",
                destination.display()
            )
        })?;
    }
    std::fs::rename(&staging, &destination).map_err(|error| {
        format!(
            "failed to install remote package cache {}: {error}",
            destination.display()
        )
    })?;
    Ok(destination)
}

pub(super) fn publish(
    package_path: &Path,
    manifest_name: &Path,
    explicit_registry: Option<&Path>,
) -> Result<PathBuf, String> {
    let checked = shared::validate_package(package_path, manifest_name, None, false)
        .map_err(|diagnostics| diagnostics.join("; "))?;
    let registry = registry_root(explicit_registry)?;
    let destination = registry
        .join(&checked.manifest.source.package)
        .join(&checked.manifest.source.version);
    if destination.exists() {
        return Err(format!(
            "registry package already exists and is immutable: {}",
            destination.display()
        ));
    }
    fs_util::copy_dir_clean(&checked.package_root, &destination)?;
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
    let source_manifest = source.join(manifest::MANIFEST_FILE);
    if !source_manifest.is_file() {
        return Err(format!(
            "package `{name}@{version}` is not published in registry {}",
            registry.display()
        ));
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
    archive
        .append_dir_all(".", root)
        .map_err(|error| format!("failed to archive package {}: {error}", root.display()))?;
    archive
        .into_inner()
        .map_err(|error| format!("failed to finish package archive: {error}"))
}

fn unpack_archive(bytes: &[u8], destination: &Path) -> Result<(), String> {
    tar::Archive::new(bytes)
        .unpack(destination)
        .map_err(|error| format!("failed to unpack remote package archive: {error}"))
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
