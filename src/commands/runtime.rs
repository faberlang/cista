use crate::cli::{CistaCommand, RuntimeCommand};

use super::{CommandResult, staged};

pub fn run(args: RuntimeCommand) -> CommandResult {
    staged::run(&CistaCommand::Runtime(args))
}
