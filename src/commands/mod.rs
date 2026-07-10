//! Low-level command dispatch for the `cista` binary.
//!
//! Each top-level CLI command has its own module with a `run` entrypoint, mirroring
//! the `CistaCommand` tree defined in [`crate::cli`].

pub(super) use std::env;
pub(super) use std::fs;
pub(super) use std::path::{Path, PathBuf};

mod cache;
mod check;
mod doctor;
mod fetch;
mod fs_util;
mod graph;
mod init;
mod inspect;
mod install;
mod login;
mod logout;
mod metadata;
mod package;
mod publish;
mod registry;
mod remove;
mod resolve;
mod run;
mod runtime;
mod rust_target;
mod shared;
mod staged;
mod target;
mod update;
mod yank;

use crate::cli::{CistaCli, CistaCommand};

/// Outcome of one `cista` command invocation.
pub type CommandResult = Result<(), Vec<String>>;

/// Run a parsed `cista` command.
pub fn run(cli: CistaCli) {
    let result = match cli.command {
        CistaCommand::Init(args) => init::run(args),
        CistaCommand::Check(args) => check::run(args),
        CistaCommand::Inspect(args) => inspect::run(args),
        CistaCommand::Metadata(args) => metadata::run(args),
        CistaCommand::Graph(args) => graph::run(args),
        CistaCommand::Resolve(args) => resolve::run(args),
        CistaCommand::Fetch(args) => fetch::run(args),
        CistaCommand::Install(args) => install::run(args),
        CistaCommand::Run(args) => run::run(args),
        CistaCommand::Remove(args) => remove::run(args),
        CistaCommand::Update(args) => update::run(args),
        CistaCommand::Cache(args) => cache::run(args),
        CistaCommand::Package(args) => package::run(args),
        CistaCommand::Runtime(args) => runtime::run(args),
        CistaCommand::Target(args) => target::run(args),
        CistaCommand::Publish(args) => publish::run(args),
        CistaCommand::Yank(args) => yank::run(args),
        CistaCommand::Login => login::run(),
        CistaCommand::Logout => logout::run(),
        CistaCommand::Doctor => doctor::run(),
    };

    if let Err(diagnostics) = result {
        for diagnostic in diagnostics {
            eprintln!("error: {diagnostic}");
        }
        std::process::exit(1);
    }
}
