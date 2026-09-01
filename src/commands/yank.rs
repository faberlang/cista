use crate::cli::{CistaCommand, YankArg};

use super::{CommandResult, staged};

pub fn run(args: YankArg) -> CommandResult {
    staged::run(&CistaCommand::Yank(args))
}
