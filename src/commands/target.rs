use crate::cli::{CistaCommand, TargetCommand};

use super::{staged, CommandResult};

pub fn run(args: TargetCommand) -> CommandResult {
    staged::run(CistaCommand::Target(args))
}
