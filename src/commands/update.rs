use crate::cli::{CistaCommand, OptionalPackageArg};

use super::{staged, CommandResult};

pub fn run(args: OptionalPackageArg) -> CommandResult {
    staged::run(&CistaCommand::Update(args))
}
