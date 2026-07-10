use crate::cli::PublishArgs;

use super::{registry, CommandResult};

pub fn run(args: PublishArgs) -> CommandResult {
    let destination = registry::publish(&args.path, &args.manifest, args.registry.as_deref())
        .map_err(|err| vec![err])?;
    println!("published: {}", destination.display());
    Ok(())
}
