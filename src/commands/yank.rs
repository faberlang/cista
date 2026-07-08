use crate::cli::{CistaCommand, YankArg};

use super::{staged, CommandResult};

pub fn run(args: YankArg) -> CommandResult {
    staged::run(CistaCommand::Yank(args))
}
