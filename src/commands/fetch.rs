use crate::cli::{CistaCommand, PackageArg};

use super::{staged, CommandResult};

pub fn run(args: PackageArg) -> CommandResult {
    staged::run(CistaCommand::Fetch(args))
}
