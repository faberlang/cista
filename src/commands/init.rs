use crate::cli::{CistaCommand, PathArg};

use super::{staged, CommandResult};

pub fn run(args: PathArg) -> CommandResult {
    staged::run(CistaCommand::Init(args))
}
