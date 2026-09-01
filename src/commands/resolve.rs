use crate::cli::{CistaCommand, ManifestArg};

use super::{CommandResult, staged};

pub fn run(args: ManifestArg) -> CommandResult {
    staged::run(&CistaCommand::Resolve(args))
}
