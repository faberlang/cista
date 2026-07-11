use crate::cli::PackageOrPathArg;
use crate::manifest::{manifest_path, read_manifest, MANIFEST_FILE};
use crate::store::{self, ResolvedInspectTarget};

use super::CommandResult;

pub fn run(args: PackageOrPathArg) -> CommandResult {
    let target = store::resolve_package_or_path(&args.value, args.store.as_deref())
        .map_err(|err| vec![err])?;
    match target {
        ResolvedInspectTarget::Path(root) => inspect_path(&root),
        ResolvedInspectTarget::Installed(package) => {
            let target_manifest =
                store::read_any_target_manifest(&package).map_err(|err| vec![err])?;
            let files =
                store::list_package_files(&package.package_root).map_err(|err| vec![err])?;

            println!("package: {}", package.name);
            println!("version: {}", package.version);
            println!("store_root: {}", package.package_root.display());
            println!("interfaces: {}", package.interfaces_dir.display());
            if let Some((path, manifest)) = target_manifest {
                println!("target_manifest: {}", path.display());
                print_manifest_summary(&manifest);
            }
            println!("files: {}", files.len());
            Ok(())
        }
    }
}

fn inspect_path(root: &std::path::Path) -> CommandResult {
    let manifest_file = manifest_path(root, Some(std::path::Path::new(MANIFEST_FILE)));
    if !manifest_file.is_file() {
        return Err(vec![format!(
            "no {} under {}",
            MANIFEST_FILE,
            root.display()
        )]);
    }
    let manifest = read_manifest(&manifest_file).map_err(|err| vec![err])?;
    println!("path: {}", root.display());
    println!("manifest: {}", manifest_file.display());
    print_manifest_summary(&manifest);
    Ok(())
}

fn print_manifest_summary(manifest: &crate::manifest::CistaManifest) {
    println!("package: {}", manifest.source.package);
    println!("version: {}", manifest.source.version);
    println!("kind: {}", manifest.source.kind.kebab_name());
    println!("target.language: {}", manifest.target.language);
    println!("target.mode: {}", manifest.target.mode.kebab_name());
    println!(
        "binding_policy: {}",
        manifest.target.binding_policy.kebab_name()
    );
    println!("bindings: {}", manifest.bindings.len());
}
