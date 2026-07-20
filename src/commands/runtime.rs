use crate::cli::{CistaCommand, RuntimeCommand};

use super::{staged, CommandResult};

pub fn run(args: RuntimeCommand) -> CommandResult {
    staged::run(&CistaCommand::Runtime(args))
}
