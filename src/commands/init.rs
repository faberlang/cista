use crate::cli::{CistaCommand, PathArg};

use super::{CommandResult, staged};

pub fn run(args: PathArg) -> CommandResult {
    staged::run(&CistaCommand::Init(args))
}
