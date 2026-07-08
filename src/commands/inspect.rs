use crate::cli::{CistaCommand, PackageOrPathArg};

use super::{staged, CommandResult};

pub fn run(args: PackageOrPathArg) -> CommandResult {
    staged::run(CistaCommand::Inspect(args))
}
