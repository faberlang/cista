use crate::cli::CistaCommand;

use super::{CommandResult, staged};

pub fn run() -> CommandResult {
    staged::run(&CistaCommand::Doctor)
}
