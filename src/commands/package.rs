use crate::cli::{CistaCommand, PackageCommand};

use super::{staged, CommandResult};

pub fn run(args: PackageCommand) -> CommandResult {
    staged::run(CistaCommand::Package(args))
}
