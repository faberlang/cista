use crate::cli::CheckArgs;

use super::{shared, CommandResult};

pub fn run(args: &CheckArgs) -> CommandResult {
    let checked = shared::validate_package(
        &args.path,
        &args.manifest,
        args.target_language.as_deref(),
        args.verify_target_build,
    )?;

    println!(
        "ok: {} {} ({} {}, target {}, binding policy {})",
        checked.manifest.source.package,
        checked.manifest.source.version,
        checked.manifest.source.kind.kebab_name(),
        checked.manifest.source.role.kebab_name(),
        checked.manifest.target.language,
        checked.manifest.target.binding_policy.kebab_name()
    );
    Ok(())
}
