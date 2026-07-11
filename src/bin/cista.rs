//! Thin binary entry point for the low-level `cista` package tool.

use cista::cli::CistaCli;
use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = CistaCli::parse();
    match cista::commands::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(diagnostics) => {
            for diagnostic in diagnostics {
                eprintln!("error: {diagnostic}");
            }
            ExitCode::FAILURE
        }
    }
}
