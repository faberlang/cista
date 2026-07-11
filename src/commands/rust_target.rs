use std::process::{Command, Output};

use crate::manifest::{CistaManifest, PackageRole};

use super::{Path, PathBuf};

pub(super) const RUST_LANGUAGE: &str = "rust";

pub(super) fn verify_target_build(
    package_root: &Path,
    manifest: &CistaManifest,
    diagnostics: &mut Vec<String>,
) {
    if manifest.target.language != RUST_LANGUAGE {
        diagnostics.push(format!(
            "--verify-target-build is only implemented for target.language = `{RUST_LANGUAGE}`; got `{}`",
            manifest.target.language
        ));
        return;
    }
    if !matches!(manifest.target.mode, crate::manifest::TargetMode::Compile) {
        diagnostics.push("--verify-target-build requires target.mode = `compile`".to_owned());
        return;
    }
    // Interfaces-only pure Faber packages have no native target.source to check.
    if manifest.target.source.is_none() {
        diagnostics.push(
            "--verify-target-build has nothing to check: package has no target.source (interfaces-only)"
                .to_owned(),
        );
        return;
    }

    let Ok((cargo_toml, _)) = rust_cargo_paths(package_root, manifest) else {
        return;
    };
    if !cargo_toml.is_file() {
        return;
    }

    if let Err(err) = run_cargo(&cargo_toml, &["check"], "cargo check") {
        diagnostics.push(err);
    }
}

pub(super) fn build_rust_artifact(
    package_root: &Path,
    manifest: &CistaManifest,
) -> Result<PathBuf, String> {
    let (cargo_toml, target_source) = rust_cargo_paths(package_root, manifest)?;
    let crate_name = manifest
        .target
        .crate_name
        .as_deref()
        .unwrap_or(&manifest.source.package);
    let (cargo_args, artifact_name) = match manifest.source.role {
        PackageRole::Lib => (
            vec!["build", "--lib"],
            format!("lib{}.rlib", crate_name.replace('-', "_")),
        ),
        PackageRole::Bin => (vec!["build", "--bin", crate_name], crate_name.to_owned()),
    };
    run_cargo(&cargo_toml, &cargo_args, "cargo build")?;

    let artifact = package_root
        .join(target_source)
        .join("target")
        .join("debug")
        .join(artifact_name);
    if !artifact.is_file() {
        return Err(format!(
            "cargo build succeeded but expected rust artifact is missing: {}",
            artifact.display()
        ));
    }
    Ok(artifact)
}

pub(super) fn rust_cargo_paths(
    package_root: &Path,
    manifest: &CistaManifest,
) -> Result<(PathBuf, PathBuf), String> {
    let Some(target_source) = manifest.target.source.clone() else {
        return Err("rust target requires target.source".to_owned());
    };
    let cargo_toml = package_root.join(&target_source).join("Cargo.toml");
    Ok((cargo_toml, target_source))
}

pub(super) fn run_cargo(cargo_toml: &Path, cargo_args: &[&str], label: &str) -> Result<(), String> {
    // Subcommand first, then --manifest-path (cargo rejects global --manifest-path
    // before the subcommand on modern toolchains).
    let mut command = Command::new("cargo");
    let (head, rest) = cargo_args
        .split_first()
        .ok_or_else(|| format!("{label}: cargo_args must include a subcommand"))?;
    command.arg(head);
    command.arg("--manifest-path").arg(cargo_toml);
    for arg in rest {
        command.arg(arg);
    }
    let status = command.status().map_err(|err| {
        format!(
            "failed to run {label} for rust target manifest {}: {err}",
            cargo_toml.display()
        )
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{label} failed for rust target manifest {} with status {status}",
            cargo_toml.display()
        ))
    }
}

pub(super) fn rust_host_triple() -> Result<String, String> {
    let output = run_rustc(&["-vV"])?;
    ensure_rustc_success(&output, "rustc -vV")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .ok_or_else(|| "rustc -vV did not report a host triple".to_owned())
}

pub(super) fn rustc_version() -> Result<String, String> {
    let output = run_rustc(&["--version"])?;
    ensure_rustc_success(&output, "rustc --version")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .split_whitespace()
        .nth(1)
        .map(str::to_owned)
        .ok_or_else(|| "rustc --version did not report a version".to_owned())
}

fn run_rustc(args: &[&str]) -> Result<Output, String> {
    Command::new("rustc")
        .args(args)
        .output()
        .map_err(|err| format!("failed to run rustc {}: {err}", args.join(" ")))
}

fn ensure_rustc_success(output: &Output, label: &str) -> Result<(), String> {
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("{label} failed with status {}", output.status))
    }
}
