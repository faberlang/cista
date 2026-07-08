//! Thin binary entry point for the low-level `cista` package tool.

use cista::cli::CistaCli;
use clap::Parser;

fn main() {
    let cli = CistaCli::parse();
    cista::commands::run(cli);
}
