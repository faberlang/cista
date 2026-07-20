//! Registry bearer credential persistence.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Default, Deserialize, Serialize)]
struct CredentialFile {
    registry: Vec<Credential>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Credential {
    origin: String,
    token: String,
}

/// Return the default `~/.faber/credentials.toml` path.
///
/// # Errors
/// Returns an error when the `HOME` environment variable is not set.
pub fn default_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| "HOME is unavailable; cannot locate registry credentials".to_owned())?;
    Ok(PathBuf::from(home).join(".faber").join("credentials.toml"))
}

/// Persist a credential entry for a registry origin.
///
/// # Errors
/// Returns an error when the origin or token fails validation, the credential
/// file cannot be read, or the write-and-replace sequence fails.
pub fn store(path: &Path, origin: &str, token: &str) -> Result<(), String> {
    validate(origin, token)?;
    let mut credentials = read(path)?;
    credentials.registry.retain(|entry| entry.origin != origin);
    credentials.registry.push(Credential {
        origin: origin.to_owned(),
        token: token.to_owned(),
    });
    write(path, &credentials)
}

/// Remove the credential entry for a registry origin.
///
/// Returns `Ok(true)` when an entry was removed and `Ok(false)` when no
/// matching entry existed.
///
/// # Errors
/// Returns an error when the origin fails validation, the credential file
/// cannot be read, or the write-and-replace sequence fails.
pub fn remove(path: &Path, origin: &str) -> Result<bool, String> {
    validate_origin(origin)?;
    let mut credentials = read(path)?;
    let previous_len = credentials.registry.len();
    credentials.registry.retain(|entry| entry.origin != origin);
    if credentials.registry.len() == previous_len {
        return Ok(false);
    }
    write(path, &credentials)?;
    Ok(true)
}

/// Look up the stored bearer token for a registry origin.
///
/// # Errors
/// Returns an error when the origin fails validation or the credential file
/// cannot be read.
pub fn token(path: &Path, origin: &str) -> Result<Option<String>, String> {
    validate_origin(origin)?;
    Ok(read(path)?
        .registry
        .into_iter()
        .find(|entry| entry.origin == origin)
        .map(|entry| entry.token))
}

fn validate(origin: &str, token: &str) -> Result<(), String> {
    validate_origin(origin)?;
    if token.trim().is_empty() {
        return Err("registry bearer token must not be empty".to_owned());
    }
    Ok(())
}

fn validate_origin(origin: &str) -> Result<(), String> {
    let uri: ureq::http::Uri = origin
        .parse()
        .map_err(|_| "registry credential origin must be a valid HTTPS origin".to_owned())?;
    let valid_authority = uri
        .authority()
        .is_some_and(|authority| !authority.as_str().contains('@'));
    if uri.scheme_str() != Some("https")
        || !valid_authority
        || uri
            .path_and_query()
            .is_some_and(|value| value.as_str() != "/")
        || origin.ends_with('/')
    {
        return Err(
            "registry credential origin must be a bare HTTPS origin without userinfo, path, query, or trailing slash"
                .to_owned(),
        );
    }
    Ok(())
}

fn read(path: &Path) -> Result<CredentialFile, String> {
    if !path.exists() {
        return Ok(CredentialFile::default());
    }
    verify_permissions(path)?;
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read credentials {}: {error}", path.display()))?;
    toml::from_str(&contents)
        .map_err(|error| format!("failed to parse credentials {}: {error}", path.display()))
}

#[cfg(unix)]
fn verify_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::metadata(path)
        .map_err(|error| format!("failed to inspect credentials {}: {error}", path.display()))?
        .permissions()
        .mode();
    if mode & 0o077 != 0 {
        return Err(format!(
            "credentials {} must not be accessible by group or other users",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn verify_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn write(path: &Path, credentials: &CredentialFile) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("credential path {} has no parent", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create credential directory {}: {error}",
            parent.display()
        )
    })?;
    let contents = toml::to_string(credentials)
        .map_err(|error| format!("failed to encode registry credentials: {error}"))?;
    let temporary = temporary_path(path);
    write_and_replace(path, &temporary, contents.as_bytes())
}

fn temporary_path(path: &Path) -> PathBuf {
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    path.with_extension(format!("tmp-{}-{sequence}", std::process::id()))
}

fn write_and_replace(path: &Path, temporary: &Path, contents: &[u8]) -> Result<(), String> {
    let mut file = secure_create(temporary)?;
    let write_result = file
        .write_all(contents)
        .and_then(|()| file.sync_all())
        .map_err(|error| {
            format!(
                "failed to write credentials {}: {error}",
                temporary.display()
            )
        });
    drop(file);
    let result = write_result.and_then(|()| {
        fs::rename(temporary, path).map_err(|error| {
            format!(
                "failed to replace credentials {} with {}: {error}",
                path.display(),
                temporary.display()
            )
        })
    });
    match result {
        Ok(()) => Ok(()),
        Err(operation_error) => match remove_temporary(temporary) {
            Ok(()) => Err(operation_error),
            Err(cleanup_error) => Err(format!("{operation_error}; {cleanup_error}")),
        },
    }
}

fn remove_temporary(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "failed to remove temporary credentials {}: {error}",
            path.display()
        )),
    }
}

#[cfg(unix)]
fn secure_create(path: &Path) -> Result<fs::File, String> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| format!("failed to create credentials {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn secure_create(path: &Path) -> Result<fs::File, String> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("failed to create credentials {}: {error}", path.display()))
}

#[cfg(test)]
#[path = "credentials_test.rs"]
mod tests;
