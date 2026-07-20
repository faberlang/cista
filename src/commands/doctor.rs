use crate::cli::CistaCommand;

use super::{staged, CommandResult};

pub fn run() -> CommandResult {
    staged::run(&CistaCommand::Doctor)
}
