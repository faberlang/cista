use crate::cli::{CistaCommand, OptionalPackageArg};

use super::{CommandResult, staged};

pub fn run(args: OptionalPackageArg) -> CommandResult {
    staged::run(&CistaCommand::Update(args))
}
