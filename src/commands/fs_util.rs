use std::sync::atomic::{AtomicU64, Ordering};

use super::{fs, Path};

static REPLACEMENT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) fn copy_dir_clean(source: &Path, destination: &Path) -> Result<(), String> {
    let sequence = REPLACEMENT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let staging = replacement_path(destination, "incoming", sequence);
    let backup = replacement_path(destination, "replaced", sequence);

    if let Err(error) = copy_dir_recursive(source, &staging) {
        remove_directory_if_present(&staging)?;
        return Err(error);
    }

    if destination.exists() {
        fs::rename(destination, &backup).map_err(|err| {
            format!(
                "failed to stage existing directory {} for replacement: {err}",
                destination.display()
            )
        })?;
    }
    if let Err(error) = fs::rename(&staging, destination) {
        if backup.exists() {
            fs::rename(&backup, destination).map_err(|rollback_error| {
                format!(
                    "failed to restore {} after replacement failed: {rollback_error}",
                    destination.display()
                )
            })?;
        }
        return Err(format!(
            "failed to install replacement directory {}: {error}",
            destination.display()
        ));
    }
    remove_directory_if_present(&backup)
}

pub(super) fn copy_dir_new(source: &Path, destination: &Path) -> Result<(), String> {
    match fs::create_dir(destination) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(format!(
                "directory already exists: {}",
                destination.display()
            ));
        }
        Err(err) => {
            return Err(format!(
                "failed to create new directory {}: {err}",
                destination.display()
            ));
        }
    }
    if let Err(error) = copy_dir_recursive(source, destination) {
        remove_directory_if_present(destination)?;
        return Err(error);
    }
    Ok(())
}

fn replacement_path(destination: &Path, state: &str, sequence: u64) -> std::path::PathBuf {
    destination.with_extension(format!("{state}-{}-{sequence}", std::process::id()))
}

fn remove_directory_if_present(path: &Path) -> Result<(), String> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "failed to remove directory {}: {err}",
            path.display()
        )),
    }
}

pub(super) fn replace_directory(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|err| {
            format!(
                "failed to replace installed target directory {}: {err}",
                path.display()
            )
        })?;
    }
    fs::create_dir_all(path).map_err(|err| {
        format!(
            "failed to create installed target directory {}: {err}",
            path.display()
        )
    })
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|err| {
        format!(
            "failed to create directory {}: {err}",
            destination.display()
        )
    })?;
    let entries = fs::read_dir(source)
        .map_err(|err| format!("failed to read directory {}: {err}", source.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            format!(
                "failed to read directory entry under {}: {err}",
                source.display()
            )
        })?;
        let file_type = entry.file_type().map_err(|err| {
            format!(
                "failed to inspect directory entry {}: {err}",
                entry.path().display()
            )
        })?;
        let destination_path = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &destination_path)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &destination_path).map_err(|err| {
                format!(
                    "failed to copy {} to {}: {err}",
                    entry.path().display(),
                    destination_path.display()
                )
            })?;
        } else {
            return Err(format!(
                "unsupported package entry {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "fs_util_test.rs"]
mod tests;
