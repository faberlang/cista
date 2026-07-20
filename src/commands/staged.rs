use crate::cli::CistaCommand;

use super::CommandResult;

pub(super) fn run(command: &CistaCommand) -> CommandResult {
    println!(
        "cista command accepted: {command:?}\nstatus: package-store operation is staged but not implemented yet"
    );
    Ok(())
}
