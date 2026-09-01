use crate::cli::{CacheCommand, CistaCommand};

use super::{CommandResult, staged};

pub fn run(args: CacheCommand) -> CommandResult {
    staged::run(&CistaCommand::Cache(args))
}
