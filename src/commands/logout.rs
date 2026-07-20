use crate::cli::RegistryOriginArg;
use crate::credentials;

use super::CommandResult;

pub fn run(args: &RegistryOriginArg) -> CommandResult {
    let path = credentials::default_path().map_err(|error| vec![error])?;
    if !credentials::remove(&path, &args.registry_url).map_err(|error| vec![error])? {
        return Err(vec![format!(
            "no registry credentials stored for {}",
            args.registry_url
        )]);
    }
    println!("logged out: {}", args.registry_url);
    Ok(())
}
