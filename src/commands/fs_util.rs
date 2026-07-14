use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(test)]
use std::cell::RefCell;

use super::{fs, Path, PathBuf};

static REPLACEMENT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
thread_local! {
    static INJECT_COMMIT_FAILURE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static INJECT_FINALIZE_SYNC_FAILURE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

pub(super) fn copy_dir_clean(source: &Path, destination: &Path) -> Result<(), String> {
    verify_disjoint_directories(source, destination)?;
    let sequence = REPLACEMENT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    copy_dir_clean_with_sequence(source, destination, sequence)
}

fn copy_dir_clean_with_sequence(
    source: &Path,
    destination: &Path,
    sequence: u64,
) -> Result<(), String> {
    let staging = replacement_path(destination, "incoming", sequence);
    create_staging_directory(&staging)?;
    if let Err(error) = copy_dir_contents(source, &staging) {
        remove_directory_if_present(&staging)?;
        return Err(error);
    }

    let mut replacement = commit_staged_directory_with_sequence(&staging, destination, sequence)?;
    replacement.finalize()
}

pub(super) fn stage_directory(destination: &Path) -> Result<PathBuf, String> {
    let sequence = REPLACEMENT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let staging = replacement_path(destination, "incoming", sequence);
    create_staging_directory(&staging)?;
    Ok(staging)
}

pub(super) fn discard_staged_directory(staging: &Path) -> Result<(), String> {
    remove_directory_if_present(staging)
}

pub(super) fn commit_staged_directory_transaction(
    staging: &Path,
    destination: &Path,
) -> Result<DirectoryReplacement, String> {
    let sequence = REPLACEMENT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    commit_staged_directory_with_sequence(staging, destination, sequence)
}

pub(super) struct DirectoryReplacement {
    destination: PathBuf,
    backup: Option<PathBuf>,
    committed: bool,
}

impl DirectoryReplacement {
    pub(super) fn finalize(&mut self) -> Result<(), String> {
        if self.committed {
            return Ok(());
        }
        let Some(backup) = self.backup.as_ref() else {
            self.committed = true;
            return Ok(());
        };
        remove_directory_if_present(backup)?;
        self.backup = None;
        self.committed = true;
        #[cfg(test)]
        if should_inject_finalize_sync_failure(&self.destination) {
            return Err(format!(
                "injected failure after committing replacement directory {}",
                self.destination.display()
            ));
        }
        sync_parent_directory(&self.destination)
    }

    pub(super) fn can_rollback(&self) -> bool {
        !self.committed
    }

    pub(super) fn rollback(&mut self) -> Result<(), String> {
        if self.committed {
            return Ok(());
        }
        remove_directory_if_present(&self.destination)?;
        let Some(backup) = self.backup.as_ref() else {
            self.committed = true;
            return sync_parent_directory(&self.destination);
        };
        restore_replaced_directory(backup, &self.destination)?;
        self.backup = None;
        self.committed = true;
        sync_parent_directory(&self.destination)
    }
}

fn commit_staged_directory_with_sequence(
    staging: &Path,
    destination: &Path,
    sequence: u64,
) -> Result<DirectoryReplacement, String> {
    sync_directory_tree(staging)?;
    let backup = replacement_path(destination, "replaced", sequence);
    let backup = if destination.exists() {
        fs::rename(destination, &backup).map_err(|err| {
            format!(
                "failed to stage existing directory {} for replacement: {err}",
                destination.display()
            )
        })?;
        Some(backup)
    } else {
        None
    };

    #[cfg(test)]
    if should_inject_commit_failure(destination) {
        if let Some(backup) = &backup {
            restore_replaced_directory(backup, destination)?;
        }
        return Err(format!(
            "injected failure before installing replacement directory {}",
            destination.display()
        ));
    }

    if let Err(error) = fs::rename(staging, destination) {
        if let Some(backup) = &backup {
            restore_replaced_directory(backup, destination)?;
        }
        return Err(format!(
            "failed to install replacement directory {}: {error}",
            destination.display()
        ));
    }
    let mut replacement = DirectoryReplacement {
        destination: destination.to_path_buf(),
        backup,
        committed: false,
    };
    if let Err(error) = sync_parent_directory(destination) {
        let rollback = replacement.rollback();
        return Err(match rollback {
            Ok(()) => error,
            Err(rollback_error) => format!("{error}; {rollback_error}"),
        });
    }
    Ok(replacement)
}

fn create_staging_directory(staging: &Path) -> Result<(), String> {
    let parent = staging
        .parent()
        .ok_or_else(|| format!("replacement directory has no parent: {}", staging.display()))?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create replacement parent directory {}: {error}",
            parent.display()
        )
    })?;
    fs::create_dir(staging).map_err(|error| {
        format!(
            "failed to reserve replacement directory {}: {error}",
            staging.display()
        )
    })
}

fn restore_replaced_directory(backup: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(backup, destination).map_err(|rollback_error| {
        format!(
            "failed to restore {} after replacement failed: {rollback_error}",
            destination.display()
        )
    })
}

#[cfg(test)]
pub(super) fn inject_commit_failure(destination: &Path) {
    INJECT_COMMIT_FAILURE.with(|failure| {
        *failure.borrow_mut() = Some(destination.to_path_buf());
    });
}

#[cfg(test)]
pub(super) fn inject_finalize_sync_failure(destination: &Path) {
    INJECT_FINALIZE_SYNC_FAILURE.with(|failure| {
        *failure.borrow_mut() = Some(destination.to_path_buf());
    });
}

#[cfg(test)]
fn should_inject_commit_failure(destination: &Path) -> bool {
    INJECT_COMMIT_FAILURE.with(|failure| {
        let mut failure = failure.borrow_mut();
        if failure
            .as_ref()
            .is_some_and(|expected| expected == destination)
        {
            failure.take();
            true
        } else {
            false
        }
    })
}

#[cfg(test)]
fn should_inject_finalize_sync_failure(destination: &Path) -> bool {
    INJECT_FINALIZE_SYNC_FAILURE.with(|failure| {
        let mut failure = failure.borrow_mut();
        if failure
            .as_ref()
            .is_some_and(|expected| expected == destination)
        {
            failure.take();
            true
        } else {
            false
        }
    })
}

fn verify_disjoint_directories(source: &Path, destination: &Path) -> Result<(), String> {
    let source = source.canonicalize().map_err(|err| {
        format!(
            "failed to resolve source directory {}: {err}",
            source.display()
        )
    })?;
    let existing_parent = destination
        .ancestors()
        .find(|ancestor| ancestor.exists())
        .ok_or_else(|| {
            format!(
                "destination directory has no existing parent: {}",
                destination.display()
            )
        })?;
    let suffix = destination.strip_prefix(existing_parent).map_err(|err| {
        format!(
            "failed to resolve destination directory {} from existing parent {}: {err}",
            destination.display(),
            existing_parent.display()
        )
    })?;
    let destination = existing_parent
        .canonicalize()
        .map(|parent| parent.join(suffix))
        .map_err(|err| {
            format!(
                "failed to resolve destination parent {}: {err}",
                existing_parent.display()
            )
        })?;
    if source.starts_with(&destination) || destination.starts_with(&source) {
        return Err(format!(
            "source and destination directories must not overlap: {} and {}",
            source.display(),
            destination.display()
        ));
    }
    Ok(())
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
    copy_dir_contents(source, destination)
}

fn copy_dir_contents(source: &Path, destination: &Path) -> Result<(), String> {
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

fn sync_directory_tree(path: &Path) -> Result<(), String> {
    let entries = fs::read_dir(path)
        .map_err(|err| format!("failed to read staged directory {}: {err}", path.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            format!(
                "failed to read staged directory entry under {}: {err}",
                path.display()
            )
        })?;
        let entry_path = entry.path();
        let file_type = entry.file_type().map_err(|err| {
            format!(
                "failed to inspect staged directory entry {}: {err}",
                entry_path.display()
            )
        })?;
        if file_type.is_dir() {
            sync_directory_tree(&entry_path)?;
        } else if file_type.is_file() {
            fs::File::open(&entry_path)
                .and_then(|file| file.sync_all())
                .map_err(|err| {
                    format!("failed to sync staged file {}: {err}", entry_path.display())
                })?;
        } else {
            return Err(format!("unsupported staged entry {}", entry_path.display()));
        }
    }
    sync_directory(path)
}

fn sync_parent_directory(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("path has no parent directory: {}", path.display()))?;
    sync_directory(parent)
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), String> {
    fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|err| format!("failed to sync directory {}: {err}", path.display()))
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
#[path = "fs_util_test.rs"]
mod tests;
