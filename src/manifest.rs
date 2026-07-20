//! Manifest models for installed and source-distributed cista packages.
//!
//! `CistaManifest` describes buildable library and binary packages.
//! `MetaManifest` is a separate dependency-set schema: it has no target,
//! interfaces, sources, or bindings. The schemas share file naming and package
//! identity conventions, but their roles are deliberately disjoint.

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

/// Minimal manifest for a dependency-set package.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetaManifest {
    pub source: MetaSourceSection,
    #[serde(default)]
    pub dependencies: Vec<MetaDependency>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetaSourceSection {
    pub package: String,
    pub version: String,
    pub role: MetaPackageRole,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetaDependency {
    pub package: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSection {
    pub package: String,
    pub version: String,
    pub faber_min: String,
    pub kind: SourceKind,
    #[serde(default)]
    pub role: PackageRole,
    pub interfaces: PathBuf,
    pub sources: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageRole {
    #[default]
    Lib,
    Bin,
}

impl PackageRole {
    /// Manifest spelling for this package role.
    #[must_use]
    pub const fn kebab_name(self) -> &'static str {
        match self {
            Self::Lib => "lib",
            Self::Bin => "bin",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MetaPackageRole {
    Meta,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    Source,
    Artifact,
}

impl SourceKind {
    /// Manifest spelling for this source kind.
    #[must_use]
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TargetMode {
    Compile,
    Artifact,
}

impl TargetMode {
    /// Manifest spelling for this target mode.
    #[must_use]
    pub const fn kebab_name(self) -> &'static str {
        match self {
            Self::Compile => "compile",
            Self::Artifact => "artifact",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BindingPolicy {
    Generated,
    Manifest,
}

impl BindingPolicy {
    /// Manifest spelling for this binding policy.
    #[must_use]
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
#[must_use]
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

/// Read a minimal meta-package manifest when `source.role = "meta"`.
pub fn read_meta_manifest(path: &Path) -> Result<Option<MetaManifest>, String> {
    let contents = fs::read_to_string(path)
        .map_err(|err| format!("failed to read manifest {}: {err}", path.display()))?;
    let value: toml::Value = toml::from_str(&contents)
        .map_err(|err| format!("failed to parse manifest {}: {err}", path.display()))?;
    let is_meta = value
        .get("source")
        .and_then(|source| source.get("role"))
        .and_then(toml::Value::as_str)
        == Some("meta");
    if !is_meta {
        return Ok(None);
    }
    toml::from_str(&contents)
        .map(Some)
        .map_err(|err| format!("failed to parse meta manifest {}: {err}", path.display()))
}

#[cfg(test)]
#[path = "manifest_test.rs"]
mod tests;
