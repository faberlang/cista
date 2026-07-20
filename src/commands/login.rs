use crate::cli::RegistryAuthArgs;
use crate::credentials;

use super::CommandResult;

pub fn run(args: &RegistryAuthArgs) -> CommandResult {
    let token = std::env::var(&args.token_env).map_err(|_| {
        vec![format!(
            "registry token environment variable `{}` is not set",
            args.token_env
        )]
    })?;
    let path = credentials::default_path().map_err(|error| vec![error])?;
    credentials::store(&path, &args.registry_url, &token).map_err(|error| vec![error])?;
    println!("authenticated: {}", args.registry_url);
    Ok(())
}
