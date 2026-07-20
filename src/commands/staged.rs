use crate::cli::CistaCommand;

use super::CommandResult;

// CommandResult is the uniform CLI trampoline type; stubs only return Ok(())
// until real implementations land.
#[allow(clippy::unnecessary_wraps)]
pub(super) fn run(command: &CistaCommand) -> CommandResult {
    println!(
        "cista command accepted: {command:?}\nstatus: package-store operation is staged but not implemented yet"
    );
    Ok(())
}
