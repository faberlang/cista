use crate::cli::PackageArg;
use crate::store;

use super::{fs, CommandResult};

pub fn run(args: PackageArg) -> CommandResult {
    let store_root = store::store_root(args.store.as_deref()).map_err(|err| vec![err])?;
    let package = store::find_installed(&store_root, &args.package).map_err(|err| vec![err])?;
    fs::remove_dir_all(&package.package_root).map_err(|err| {
        vec![format!(
            "failed to remove {}: {err}",
            package.package_root.display()
        )]
    })?;

    // Drop empty package name directory when no versions remain.
    if let Some(name_dir) = package.package_root.parent() {
        if name_dir
            .read_dir()
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false)
        {
            let _ = fs::remove_dir(name_dir);
        }
    }

    println!(
        "removed: {}@{} ({})",
        package.name,
        package.version,
        package.package_root.display()
    );
    Ok(())
}
