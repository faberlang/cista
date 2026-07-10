use std::process::Command;

use crate::cli::RunArgs;
use crate::manifest::PackageRole;
use crate::{manifest, store};

use super::{rust_target, CommandResult, Path, PathBuf};

pub fn run(args: RunArgs) -> CommandResult {
    let store_root = store::store_root(args.store.as_deref()).map_err(|err| vec![err])?;
    let package = store::find_installed(&store_root, &args.package).map_err(|err| vec![err])?;
    let host = rust_target::rust_host_triple().map_err(|err| vec![err])?;
    let target_root = package.targets_dir.join("rust").join(&host);
    let manifest_path = target_root.join(manifest::MANIFEST_FILE);
    let installed = manifest::read_manifest(&manifest_path).map_err(|err| {
        vec![format!(
            "package `{}@{}` has no runnable Rust target for host `{host}`: {err}",
            package.name, package.version
        )]
    })?;
    let executable = executable_path(&installed, &target_root, &host).map_err(|err| vec![err])?;
    let status = Command::new(&executable)
        .args(&args.args)
        .status()
        .map_err(|err| vec![format!("failed to run {}: {err}", executable.display())])?;
    if status.success() {
        Ok(())
    } else {
        Err(vec![format!(
            "installed executable {} exited with status {status}",
            executable.display()
        )])
    }
}

fn executable_path(
    installed: &manifest::CistaManifest,
    target_root: &Path,
    host: &str,
) -> Result<PathBuf, String> {
    if !matches!(installed.source.role, PackageRole::Bin) {
        return Err(format!(
            "package `{}@{}` has role `{}`; only `bin` packages can be run",
            installed.source.package,
            installed.source.version,
            installed.source.role.kebab_name()
        ));
    }
    if installed.target.language != rust_target::RUST_LANGUAGE
        || installed.target.triple.as_deref() != Some(host)
    {
        return Err(format!(
            "package `{}@{}` target metadata does not match Rust host `{host}`",
            installed.source.package, installed.source.version
        ));
    }
    let artifact = installed
        .target
        .artifact
        .as_deref()
        .ok_or_else(|| "installed binary manifest is missing target.artifact".to_owned())?;
    if artifact.components().count() != 1 {
        return Err(format!(
            "installed binary artifact must be a file name, got `{}`",
            artifact.display()
        ));
    }
    let executable = target_root.join(artifact);
    if !executable.is_file() {
        return Err(format!(
            "installed binary artifact is missing: {}",
            executable.display()
        ));
    }
    Ok(executable)
}

#[cfg(test)]
#[path = "run_test.rs"]
mod tests;
