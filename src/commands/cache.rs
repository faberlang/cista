use crate::cli::{CacheCommand, CistaCommand};

use super::{staged, CommandResult};

pub fn run(args: CacheCommand) -> CommandResult {
    staged::run(&CistaCommand::Cache(args))
}
