use crate::cli::PackageArg;
use crate::store;

use super::{fs, shared, CommandResult};
use std::io::ErrorKind;

pub fn run(args: &PackageArg) -> CommandResult {
    let store_root = store::store_root(args.store.as_deref()).map_err(|err| vec![err])?;
    let mutation_locks =
        shared::acquire_store_mutation_locks(&store_root, None).map_err(|error| vec![error])?;
    let package =
        store::find_verified_installed(&store_root, &args.package).map_err(|err| vec![err])?;
    fs::remove_dir_all(&package.package_root).map_err(|err| {
        vec![format!(
            "failed to remove {}: {err}",
            package.package_root.display()
        )]
    })?;

    if let Some(name_dir) = package.package_root.parent() {
        remove_empty_name_dir(name_dir).map_err(|err| vec![err])?;
    }

    println!(
        "removed: {}@{} ({})",
        package.name,
        package.version,
        package.package_root.display()
    );
    drop(mutation_locks);
    Ok(())
}

fn remove_empty_name_dir(name_dir: &std::path::Path) -> Result<(), String> {
    let mut entries = name_dir.read_dir().map_err(|error| {
        format!(
            "failed to inspect package directory {} after removal: {error}",
            name_dir.display()
        )
    })?;
    if entries.next().is_some() {
        return Ok(());
    }

    match fs::remove_dir(name_dir) {
        Ok(()) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::NotFound | ErrorKind::DirectoryNotEmpty
            ) =>
        {
            Ok(())
        }
        Err(error) => Err(format!(
            "failed to remove empty package directory {}: {error}",
            name_dir.display()
        )),
    }
}

#[cfg(test)]
#[path = "remove_test.rs"]
mod tests;
