use std::fs;

use crate::manifest::{
    BindingPolicy, CistaManifest, CompileSection, PackageRole, SourceKind, SourceSection,
    TargetMode, TargetSection,
};

use super::*;

fn temp_dir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "cista-rust-target-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn compile_manifest(source: Option<&str>) -> CistaManifest {
    CistaManifest {
        source: SourceSection {
            package: "example".to_owned(),
            version: "0.1.0".to_owned(),
            faber_min: "0.38.0".to_owned(),
            kind: SourceKind::Source,
            role: PackageRole::Lib,
            interfaces: PathBuf::from("interfaces"),
            sources: None,
        },
        target: TargetSection {
            language: "rust".to_owned(),
            mode: TargetMode::Compile,
            binding_policy: BindingPolicy::Generated,
            source: source.map(PathBuf::from),
            artifact: None,
            crate_name: Some("example".to_owned()),
            triple: None,
            rustc: None,
            flags: None,
            compile: Some(CompileSection {
                emit: "library".to_owned(),
                crate_type: "rlib".to_owned(),
                edition: "2021".to_owned(),
            }),
        },
        bindings: Vec::new(),
    }
}

// --- verify_target_build ---

#[test]
fn verify_target_build_rejects_non_rust_language() {
    let mut manifest = compile_manifest(Some("target"));
    manifest.target.language = "python".to_owned();

    let mut diagnostics = Vec::new();
    verify_target_build(&manifest, None, &mut diagnostics);

    assert!(diagnostics.iter().any(|d| d.contains("only implemented for target.language")));
}

#[test]
fn verify_target_build_rejects_non_compile_mode() {
    let mut manifest = compile_manifest(Some("target"));
    manifest.target.mode = TargetMode::Artifact;

    let mut diagnostics = Vec::new();
    verify_target_build(&manifest, None, &mut diagnostics);

    assert!(diagnostics.iter().any(|d| d.contains("requires target.mode")));
}

#[test]
fn verify_target_build_rejects_missing_target_source() {
    let manifest = compile_manifest(None);

    let mut diagnostics = Vec::new();
    verify_target_build(&manifest, None, &mut diagnostics);

    assert!(diagnostics.iter().any(|d| d.contains("nothing to check")));
}

#[test]
fn verify_target_build_skips_cargo_when_no_cargo_toml() {
    let root = temp_dir("no-cargo-toml");
    let target_source = root.join("target");
    fs::create_dir_all(&target_source).expect("create target dir");

    let manifest = compile_manifest(Some("target"));

    let mut diagnostics = Vec::new();
    verify_target_build(
        &manifest,
        Some(&target_source),
        &mut diagnostics,
    );

    assert!(
        diagnostics.is_empty(),
        "no diagnostics expected when Cargo.toml is absent: {diagnostics:?}"
    );
    fs::remove_dir_all(root).expect("cleanup");
}

// --- run_cargo ---

#[test]
fn run_cargo_rejects_empty_args() {
    let root = temp_dir("run-cargo-empty");
    let cargo_toml = root.join("Cargo.toml");
    fs::write(&cargo_toml, "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n")
        .expect("write Cargo.toml");

    let error = run_cargo(&cargo_toml, &[], "test").expect_err("empty args must be rejected");
    assert!(error.contains("cargo_args must include a subcommand"));
    fs::remove_dir_all(root).expect("cleanup");
}

// --- contained_cargo_manifest ---

#[test]
fn contained_cargo_manifest_returns_none_when_cargo_toml_is_missing() {
    let root = temp_dir("missing-cargo");
    fs::create_dir_all(&root).expect("create dir");

    let result = contained_cargo_manifest(&root).expect("missing Cargo.toml should return Ok(None)");
    assert!(result.is_none());
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn contained_cargo_manifest_resolves_valid_cargo_toml() {
    let root = temp_dir("valid-cargo");
    fs::create_dir_all(&root).expect("create dir");
    let cargo_toml = root.join("Cargo.toml");
    fs::write(&cargo_toml, "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n")
        .expect("write Cargo.toml");

    // Canonicalize the root so that starts_with checks work correctly
    // when /tmp or /var is a symlink (e.g., /var -> /private/var on macOS).
    let canonical_root = root.canonicalize().expect("canonicalize root");
    let result = contained_cargo_manifest(&canonical_root).expect("valid Cargo.toml should resolve");
    assert!(result.is_some());
    let resolved = result.unwrap();
    assert!(resolved.ends_with("Cargo.toml"));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn contained_cargo_manifest_rejects_cargo_toml_outside_target_source() {
    let root = temp_dir("outside-cargo");
    let target_source = root.join("target");
    let outside = root.join("outside");
    fs::create_dir_all(&target_source).expect("create target dir");
    fs::create_dir_all(&outside).expect("create outside dir");
    let outside_cargo = outside.join("Cargo.toml");
    fs::write(&outside_cargo, "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n")
        .expect("write Cargo.toml");

    // Create a symlink from target/Cargo.toml -> ../outside/Cargo.toml
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside_cargo, target_source.join("Cargo.toml"))
            .expect("create symlink");
        let error =
            contained_cargo_manifest(&target_source).expect_err("escaping Cargo.toml must be rejected");
        assert!(error.contains("resolves outside target.source"));
    }
    fs::remove_dir_all(root).expect("cleanup");
}

// --- build_rust_artifact ---

#[test]
fn build_rust_artifact_rejects_missing_cargo_toml() {
    let root = temp_dir("build-missing-cargo");
    let target_source = root.join("target");
    fs::create_dir_all(&target_source).expect("create target dir");

    let manifest = compile_manifest(Some("target"));
    let error =
        build_rust_artifact(&target_source, &manifest).expect_err("missing Cargo.toml must be rejected");
    assert!(error.contains("missing Cargo.toml"));
    fs::remove_dir_all(root).expect("cleanup");
}
