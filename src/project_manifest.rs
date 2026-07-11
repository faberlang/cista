//! Minimal project `faber.toml` parse for package-manager lock rewrites.
//!
//! Cista only needs package identity and the `[dependencies]` table. It does
//! not share types with the faber crate.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub const PROJECT_MANIFEST: &str = "faber.toml";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectManifest {
    pub package: ProjectPackage,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    /// Accept remaining tables without interpreting them.
    #[serde(default)]
    pub paths: Option<toml::Value>,
    #[serde(default)]
    pub build: Option<toml::Value>,
    #[serde(default)]
    pub reader: Option<toml::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectPackage {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub edition: Option<String>,
}

/// Read a project manifest for dependency declaration checks.
pub fn read_project_manifest(path: &Path) -> Result<ProjectManifest, String> {
    let contents = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    // Allow unknown top-level keys by using a looser parse if deny_unknown fails.
    // Prefer strict first; fall back to value extraction for forward-compatible faber.toml.
    match toml::from_str::<ProjectManifest>(&contents) {
        Ok(manifest) => Ok(manifest),
        Err(_) => parse_dependencies_loose(&contents, path),
    }
}

fn parse_dependencies_loose(contents: &str, path: &Path) -> Result<ProjectManifest, String> {
    let value: toml::Value =
        toml::from_str(contents).map_err(|err| format!("invalid {}: {err}", path.display()))?;
    let table = value
        .as_table()
        .ok_or_else(|| format!("invalid {}: root must be a table", path.display()))?;
    let package_table = table
        .get("package")
        .and_then(|v| v.as_table())
        .ok_or_else(|| format!("{} missing [package]", path.display()))?;
    let name = package_table
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("{} package.name is required", path.display()))?
        .to_owned();
    let mut dependencies = BTreeMap::new();
    if let Some(dependencies_value) = table.get("dependencies") {
        let deps = dependencies_value
            .as_table()
            .ok_or_else(|| format!("{} [dependencies] must be a table", path.display()))?;
        for (key, val) in deps {
            let version = match val {
                toml::Value::String(s) => s.clone(),
                other => other
                    .get("version")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        format!(
                            "{} dependency `{key}` must be an exact version string",
                            path.display()
                        )
                    })?
                    .to_owned(),
            };
            dependencies.insert(key.clone(), version);
        }
    }
    Ok(ProjectManifest {
        package: ProjectPackage {
            name,
            version: package_table
                .get("version")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
            edition: package_table
                .get("edition")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
        },
        dependencies,
        paths: None,
        build: None,
        reader: None,
    })
}

/// Require an exact dependency pin for lock rewrite.
pub fn require_exact_dependency(
    manifest: &ProjectManifest,
    package: &str,
    version: &str,
) -> Result<(), String> {
    match manifest.dependencies.get(package) {
        Some(declared) if declared == version => Ok(()),
        Some(declared) => Err(format!(
            "project declares `{package} = \"{declared}\"` but installed version is `{version}`"
        )),
        None => Err(format!(
            "package `{package}` is not declared in faber.toml [dependencies]; add `{package} = \"{version}\"` before install lock rewrite"
        )),
    }
}

#[cfg(test)]
#[path = "project_manifest_test.rs"]
mod tests;
