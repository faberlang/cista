use crate::cli::PackageArg;

use super::{registry, CommandResult};

pub fn run(args: PackageArg) -> CommandResult {
    let path = if let Some(origin) = &args.registry_url {
        registry::fetch_remote_to_cache(&args.package, origin, args.store.as_deref())
    } else {
        registry::fetch_to_cache(
            &args.package,
            args.registry.as_deref(),
            args.store.as_deref(),
        )
    }
    .map_err(|err| vec![err])?;
    println!("fetched: {} -> {}", args.package, path.display());
    Ok(())
}
