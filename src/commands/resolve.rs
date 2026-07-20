use crate::cli::{CistaCommand, ManifestArg};

use super::{staged, CommandResult};

pub fn run(args: ManifestArg) -> CommandResult {
    staged::run(&CistaCommand::Resolve(args))
}
