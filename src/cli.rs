//! Clap command shapes for the low-level `cista` package tool.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// Root parser for the `cista` binary.
#[derive(Parser, Debug)]
#[command(
    name = "cista",
    bin_name = "cista",
    about = "Faber package-store and runtime binding tool",
    version
)]
pub struct CistaCli {
    /// Low-level package command selected by the caller.
    #[command(subcommand)]
    pub command: CistaCommand,
}

/// Low-level package-store and runtime command grammar.
#[derive(Subcommand, Debug)]
pub enum CistaCommand {
    /// Create a low-level package skeleton
    Init(PathArg),

    /// Validate package manifest, interfaces, runtime bindings, and resolver metadata
    Check(CheckArgs),

    /// Inspect a package path or package identifier
    Inspect(PackageOrPathArg),

    /// Emit machine-readable package metadata
    Metadata(PackageOrPathArg),

    /// Print the resolved package/provider graph
    Graph(ManifestArg),

    /// Resolve package dependencies and runtime bindings without compiling
    Resolve(ManifestArg),

    /// Fetch package metadata/artifacts into the cache
    Fetch(PackageArg),

    /// Install a local package source into the shared package store
    Install(InstallArgs),

    /// Run an executable from an installed binary package
    Run(RunArgs),

    /// Remove a package from the selected package root/cache
    Remove(PackageArg),

    /// Refresh package metadata and selected cached artifacts
    Update(OptionalPackageArg),

    /// Inspect or maintain the package cache
    Cache(CacheCommand),

    /// Inspect package contents
    Package(PackageCommand),

    /// Inspect target-native runtime bindings
    Runtime(RuntimeCommand),

    /// Inspect package target metadata
    Target(TargetCommand),

    /// Publish a package to a registry
    Publish(PublishArgs),

    /// Mark a published package version as yanked
    Yank(YankArg),

    /// Authenticate to a package registry
    Login(RegistryAuthArgs),

    /// Remove package registry credentials
    Logout(RegistryOriginArg),

    /// Run package-store health checks
    Doctor,
}

/// Registry authentication arguments. Tokens are read from the environment,
/// never from command-line values.
#[derive(Args, Debug)]
pub struct RegistryAuthArgs {
    /// HTTPS registry origin
    #[arg(long, default_value = "https://cista.dev")]
    pub registry_url: String,

    /// Environment variable containing the bearer token
    #[arg(long, default_value = "CISTA_REGISTRY_TOKEN")]
    pub token_env: String,
}

/// Registry origin selection for credential removal.
#[derive(Args, Debug)]
pub struct RegistryOriginArg {
    /// HTTPS registry origin
    #[arg(long, default_value = "https://cista.dev")]
    pub registry_url: String,
}

/// Cache subcommands.
#[derive(Args, Debug)]
pub struct CacheCommand {
    #[command(subcommand)]
    pub command: CacheSubcommand,
}

/// Cache operation grammar.
#[derive(Subcommand, Debug)]
pub enum CacheSubcommand {
    /// List cached packages and runtime artifacts
    List,
    /// Print the package cache path
    Path,
    /// Remove unreachable cache entries
    Prune,
    /// Clear package cache entries
    Clean,
}

/// Package inspection subcommands.
#[derive(Args, Debug)]
pub struct PackageCommand {
    #[command(subcommand)]
    pub command: PackageSubcommand,
}

/// Package inspection grammar.
#[derive(Subcommand, Debug)]
pub enum PackageSubcommand {
    /// List packages visible in the selected store
    List(StoreArg),
    /// Show package identity, source, version, interfaces, and targets
    Show(PackageArg),
    /// List files owned by a package
    Files(PackageArg),
    /// List Faber interface files exposed by a package
    Interfaces(PackageArg),
    /// List runtime bindings exposed by a package
    Runtimes(PackageArg),
}

/// Optional store root argument.
#[derive(Args, Debug)]
pub struct StoreArg {
    /// Shared cista package artifact store; falls back to `CISTAE_HOME`, then ~/.faber/cistae
    #[arg(long)]
    pub store: Option<PathBuf>,
}

/// Runtime inspection subcommands.
#[derive(Args, Debug)]
pub struct RuntimeCommand {
    #[command(subcommand)]
    pub command: RuntimeSubcommand,
}

/// Runtime inspection grammar.
#[derive(Subcommand, Debug)]
pub enum RuntimeSubcommand {
    /// List runtime bindings visible in the selected store/cache
    List,
    /// Show target-native runtime metadata for one package and target
    Show(PackageTargetArg),
    /// Validate runtime binding metadata for one package and target
    Verify(PackageTargetArg),
    /// Show Faber-symbol to native-symbol bindings
    Bindings(PackageTargetArg),
}

/// Target metadata subcommands.
#[derive(Args, Debug)]
pub struct TargetCommand {
    #[command(subcommand)]
    pub command: TargetSubcommand,
}

/// Target metadata grammar.
#[derive(Subcommand, Debug)]
pub enum TargetSubcommand {
    /// List supported target identifiers known to package metadata
    List,
    /// Show target metadata relevant to packages and runtimes
    Show(TargetArg),
    /// Validate target metadata in the selected package metadata
    Verify(TargetArg),
}

/// Path argument.
#[derive(Args, Debug)]
pub struct PathArg {
    /// Path to inspect or create
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

/// Package validation arguments.
#[derive(Args, Debug)]
pub struct CheckArgs {
    /// Package root containing cista.toml
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Manifest file name or relative path inside the package root
    #[arg(long, default_value = "cista.toml")]
    pub manifest: PathBuf,

    /// Expected target language; fails if it does not match [target].language
    #[arg(long)]
    pub target_language: Option<String>,

    /// Verify the selected target implementation can be built by its native tool
    #[arg(long)]
    pub verify_target_build: bool,
}

/// Local package install arguments.
#[derive(Args, Debug)]
pub struct InstallArgs {
    /// Local package root containing cista.toml
    #[arg(long, required_unless_present = "package", conflicts_with = "package")]
    pub path: Option<PathBuf>,

    /// Exact registry package identifier (`name@version`) when --path is omitted
    #[arg(required_unless_present = "path", conflicts_with = "path")]
    pub package: Option<String>,

    /// Manifest file name or relative path inside the package root
    #[arg(long, default_value = "cista.toml")]
    pub manifest: PathBuf,

    /// Target language to install
    #[arg(long)]
    pub target_language: String,

    /// Shared cista package artifact store; falls back to `CISTAE_HOME`, then ~/.faber/cistae
    #[arg(long)]
    pub store: Option<PathBuf>,

    /// Local/dev registry root; falls back to `CISTA_REGISTRY`
    #[arg(long)]
    pub registry: Option<PathBuf>,

    /// Project root containing faber.toml; when set (or when cwd has faber.toml), rewrite faber.lock
    #[arg(long)]
    pub project: Option<PathBuf>,

    /// Verify the selected target implementation can be built by its native tool before install
    #[arg(long)]
    pub verify_target_build: bool,
}

/// Local/dev registry publication arguments.
#[derive(Args, Debug)]
pub struct PublishArgs {
    /// Package source root containing cista.toml
    #[arg(long)]
    pub path: PathBuf,

    /// Manifest file name or relative path inside the package root
    #[arg(long, default_value = "cista.toml")]
    pub manifest: PathBuf,

    /// Local/dev registry root; falls back to `CISTA_REGISTRY`
    #[arg(long)]
    pub registry: Option<PathBuf>,

    /// Remote HTTPS registry origin; mutually exclusive with --registry
    #[arg(long, conflicts_with = "registry")]
    pub registry_url: Option<String>,
}

/// Installed binary execution arguments.
#[derive(Args, Debug)]
pub struct RunArgs {
    /// Installed package identifier (`name` or `name@version`)
    pub package: String,

    /// Shared cista package artifact store; falls back to `CISTAE_HOME`, then ~/.faber/cistae
    #[arg(long)]
    pub store: Option<PathBuf>,

    /// Arguments passed to the installed executable after `--`
    #[arg(last = true)]
    pub args: Vec<String>,
}

/// Package identifier argument.
#[derive(Args, Debug)]
pub struct PackageArg {
    /// Package identifier (`name` or `name@version`)
    pub package: String,

    /// Shared cista package artifact store; falls back to `CISTAE_HOME`, then ~/.faber/cistae
    #[arg(long)]
    pub store: Option<PathBuf>,

    /// Local/dev registry root; falls back to `CISTA_REGISTRY`
    #[arg(long)]
    pub registry: Option<PathBuf>,

    /// Remote HTTPS registry origin; mutually exclusive with --registry
    #[arg(long, conflicts_with = "registry")]
    pub registry_url: Option<String>,
}

/// Optional package identifier argument.
#[derive(Args, Debug)]
pub struct OptionalPackageArg {
    /// Optional package identifier
    pub package: Option<String>,
}

/// Package or filesystem path argument.
#[derive(Args, Debug)]
pub struct PackageOrPathArg {
    /// Package identifier or filesystem path
    pub value: String,

    /// Shared cista package artifact store; falls back to `CISTAE_HOME`, then ~/.faber/cistae
    #[arg(long)]
    pub store: Option<PathBuf>,
}

/// Manifest path argument.
#[derive(Args, Debug)]
pub struct ManifestArg {
    /// Manifest path
    #[arg(default_value = ".")]
    pub manifest: PathBuf,
}

/// Target identifier argument.
#[derive(Args, Debug)]
pub struct TargetArg {
    /// Target identifier
    pub target: String,
}

/// Package and target argument.
#[derive(Args, Debug)]
pub struct PackageTargetArg {
    /// Package identifier
    pub package: String,

    /// Target identifier
    #[arg(long)]
    pub target: String,
}

/// Yank command arguments.
#[derive(Args, Debug)]
pub struct YankArg {
    /// Package identifier
    pub package: String,

    /// Package version to yank
    pub version: String,
}

#[cfg(test)]
#[path = "cli_test.rs"]
mod tests;
