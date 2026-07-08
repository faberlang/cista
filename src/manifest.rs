//! Manifest model for installed and source-distributed cista packages.
//!
//! The first schema is intentionally narrow. It models the current
//! package-management experiment rather than trying to accept every future
//! registry shape: one Faber-facing `[source]`, one selected `[target]`, and
//! optional structured `[[bindings]]` rows for manifest-bound target symbols.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const MANIFEST_FILE: &str = "cista.toml";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CistaManifest {
    pub source: SourceSection,
    pub target: TargetSection,
    #[serde(default)]
    pub bindings: Vec<Binding>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSection {
    pub package: String,
    pub version: String,
    pub faber_min: String,
    pub kind: SourceKind,
    pub interfaces: PathBuf,
    pub sources: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    Source,
    Artifact,
}

impl SourceKind {
    /// Manifest spelling for this source kind.
    pub const fn kebab_name(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Artifact => "artifact",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetSection {
    pub language: String,
    pub mode: TargetMode,
    pub binding_policy: BindingPolicy,
    pub source: Option<PathBuf>,
    pub artifact: Option<PathBuf>,
    #[serde(rename = "crate")]
    pub crate_name: Option<String>,
    pub triple: Option<String>,
    pub rustc: Option<String>,
    pub flags: Option<TargetFlags>,
    pub compile: Option<CompileSection>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TargetMode {
    Compile,
    Artifact,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BindingPolicy {
    Generated,
    Manifest,
}

impl BindingPolicy {
    /// Manifest spelling for this binding policy.
    pub const fn kebab_name(self) -> &'static str {
        match self {
            Self::Generated => "generated",
            Self::Manifest => "manifest",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetFlags {
    pub edition: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompileSection {
    pub emit: String,
    pub crate_type: String,
    pub edition: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub source_module: String,
    pub source_symbol: String,
    pub target: String,
}

/// Resolve the manifest path inside a package root.
pub fn manifest_path(package_root: &Path, manifest_name: Option<&Path>) -> PathBuf {
    package_root.join(manifest_name.unwrap_or_else(|| Path::new(MANIFEST_FILE)))
}

/// Read and parse a `cista.toml` manifest from disk.
pub fn read_manifest(path: &Path) -> Result<CistaManifest, String> {
    let contents = fs::read_to_string(path)
        .map_err(|err| format!("failed to read manifest {}: {err}", path.display()))?;
    toml::from_str(&contents)
        .map_err(|err| format!("failed to parse manifest {}: {err}", path.display()))
}
