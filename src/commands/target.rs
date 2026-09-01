use crate::cli::{CistaCommand, TargetCommand};

use super::{CommandResult, staged};

pub fn run(args: TargetCommand) -> CommandResult {
    staged::run(&CistaCommand::Target(args))
}
