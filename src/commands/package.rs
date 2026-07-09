use crate::cli::{PackageCommand, PackageSubcommand};
use crate::store;

use super::CommandResult;

pub fn run(args: PackageCommand) -> CommandResult {
    match args.command {
        PackageSubcommand::List(store_arg) => list(store_arg.store.as_deref()),
        PackageSubcommand::Show(arg) => show(&arg.package, arg.store.as_deref()),
        PackageSubcommand::Files(arg) => files(&arg.package, arg.store.as_deref()),
        PackageSubcommand::Interfaces(arg) => interfaces(&arg.package, arg.store.as_deref()),
        PackageSubcommand::Runtimes(arg) => runtimes(&arg.package, arg.store.as_deref()),
    }
}

fn list(store: Option<&std::path::Path>) -> CommandResult {
    let store_root = store::store_root(store).map_err(|err| vec![err])?;
    let packages = store::list_installed(&store_root).map_err(|err| vec![err])?;
    if packages.is_empty() {
        println!("(no packages installed in {})", store_root.display());
        return Ok(());
    }
    for package in packages {
        println!("{}@{}", package.name, package.version);
    }
    Ok(())
}

fn show(package_id: &str, store: Option<&std::path::Path>) -> CommandResult {
    let store_root = store::store_root(store).map_err(|err| vec![err])?;
    let package = store::find_installed(&store_root, package_id).map_err(|err| vec![err])?;
    println!("package: {}", package.name);
    println!("version: {}", package.version);
    println!("root: {}", package.package_root.display());
    println!("interfaces: {}", package.interfaces_dir.display());
    println!("targets: {}", package.targets_dir.display());
    if let Some((path, manifest)) = store::read_any_target_manifest(&package) {
        println!("target_manifest: {}", path.display());
        println!("target.language: {}", manifest.target.language);
        println!("target.mode: {}", manifest.target.mode.kebab_name());
        println!(
            "binding_policy: {}",
            manifest.target.binding_policy.kebab_name()
        );
        if let Some(triple) = &manifest.target.triple {
            println!("target.triple: {triple}");
        }
        if let Some(artifact) = &manifest.target.artifact {
            println!("artifact: {}", artifact.display());
        }
        println!("bindings: {}", manifest.bindings.len());
    }
    Ok(())
}

fn files(package_id: &str, store: Option<&std::path::Path>) -> CommandResult {
    let store_root = store::store_root(store).map_err(|err| vec![err])?;
    let package = store::find_installed(&store_root, package_id).map_err(|err| vec![err])?;
    let files = store::list_package_files(&package.package_root).map_err(|err| vec![err])?;
    for file in files {
        println!("{}", file.display());
    }
    Ok(())
}

fn interfaces(package_id: &str, store: Option<&std::path::Path>) -> CommandResult {
    let store_root = store::store_root(store).map_err(|err| vec![err])?;
    let package = store::find_installed(&store_root, package_id).map_err(|err| vec![err])?;
    let files = store::list_package_files(&package.package_root).map_err(|err| vec![err])?;
    for file in files {
        let as_str = file.to_string_lossy();
        if as_str.starts_with("interfaces/") && as_str.ends_with(".fab") {
            println!("{as_str}");
        }
    }
    Ok(())
}

fn runtimes(package_id: &str, store: Option<&std::path::Path>) -> CommandResult {
    let store_root = store::store_root(store).map_err(|err| vec![err])?;
    let package = store::find_installed(&store_root, package_id).map_err(|err| vec![err])?;
    if let Some((_, manifest)) = store::read_any_target_manifest(&package) {
        for binding in &manifest.bindings {
            println!(
                "{}#{} -> {}",
                binding.source_module, binding.source_symbol, binding.target
            );
        }
    } else {
        println!("(no target manifest with bindings found)");
    }
    Ok(())
}
