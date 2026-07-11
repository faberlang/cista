use super::{fs, Path};

pub(super) fn copy_dir_clean(source: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        fs::remove_dir_all(destination).map_err(|err| {
            format!(
                "failed to replace directory {}: {err}",
                destination.display()
            )
        })?;
    }
    copy_dir_recursive(source, destination)
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
