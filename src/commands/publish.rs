use crate::cli::PublishArgs;

use super::{registry, CommandResult};

pub fn run(args: PublishArgs) -> CommandResult {
    if let Some(origin) = &args.registry_url {
        registry::publish_remote(&args.path, &args.manifest, origin).map_err(|err| vec![err])?;
        println!("published: {origin}");
    } else {
        let destination = registry::publish(&args.path, &args.manifest, args.registry.as_deref())
            .map_err(|err| vec![err])?;
        println!("published: {}", destination.display());
    }
    Ok(())
}
