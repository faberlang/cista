use super::*;
use crate::manifest::{Binding, PackageRole, SourceSection, TargetSection};

fn manifest(kind: SourceKind, mode: TargetMode) -> CistaManifest {
    CistaManifest {
        source: SourceSection {
            package: "example".to_owned(),
            version: "0.1.0".to_owned(),
            faber_min: "0.38.0".to_owned(),
            kind,
            role: PackageRole::Lib,
            interfaces: PathBuf::from("interfaces"),
            sources: None,
        },
        target: TargetSection {
            language: "rust".to_owned(),
            mode,
            binding_policy: BindingPolicy::Generated,
            source: None,
            artifact: Some(PathBuf::from("libexample.rlib")),
            crate_name: Some("example".to_owned()),
            triple: None,
            rustc: None,
            flags: None,
            compile: None,
        },
        bindings: Vec::new(),
    }
}

#[test]
fn manifest_shape_rejects_source_kind_with_artifact_mode() {
    let mut diagnostics = Vec::new();
    validate_manifest_shape(
        &manifest(SourceKind::Source, TargetMode::Artifact),
        &mut diagnostics,
    );

    assert!(diagnostics.iter().any(|diagnostic| diagnostic
        == "source.kind `source` is incompatible with target.mode `artifact`"));
}

#[test]
fn manifest_shape_rejects_artifact_kind_with_compile_mode() {
    let mut diagnostics = Vec::new();
    validate_manifest_shape(
        &manifest(SourceKind::Artifact, TargetMode::Compile),
        &mut diagnostics,
    );

    assert!(diagnostics.iter().any(|diagnostic| diagnostic
        == "source.kind `artifact` is incompatible with target.mode `compile`"));
}

#[test]
fn manifest_shape_rejects_sources_for_artifact_kind() {
    let mut manifest = manifest(SourceKind::Artifact, TargetMode::Artifact);
    manifest.source.sources = Some(PathBuf::from("src"));
    let mut diagnostics = Vec::new();
    validate_manifest_shape(&manifest, &mut diagnostics);

    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic == "source kind `artifact` forbids source.sources"));
}

#[test]
fn manifest_shape_rejects_artifact_field_for_compile_mode() {
    let mut diagnostics = Vec::new();
    validate_manifest_shape(
        &manifest(SourceKind::Source, TargetMode::Compile),
        &mut diagnostics,
    );

    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic == "target mode `compile` forbids target.artifact"));
}

#[test]
fn manifest_shape_rejects_compile_fields_for_artifact_mode() {
    let mut manifest = manifest(SourceKind::Artifact, TargetMode::Artifact);
    manifest.target.source = Some(PathBuf::from("target"));
    manifest.target.compile = Some(crate::manifest::CompileSection {
        emit: "library".to_owned(),
        crate_type: "rlib".to_owned(),
        edition: "2021".to_owned(),
    });
    let mut diagnostics = Vec::new();
    validate_manifest_shape(&manifest, &mut diagnostics);

    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic == "target mode `artifact` forbids target.source"));
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic == "target mode `artifact` forbids [target.compile]"));
}

#[test]
fn manifest_shape_rejects_artifact_provenance_for_compile_mode() {
    let mut manifest = manifest(SourceKind::Source, TargetMode::Compile);
    manifest.target.artifact = None;
    manifest.target.triple = Some("aarch64-apple-darwin".to_owned());
    manifest.target.rustc = Some("rustc 1.88.0".to_owned());
    let mut diagnostics = Vec::new();
    validate_manifest_shape(&manifest, &mut diagnostics);

    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic == "target mode `compile` forbids target.triple"));
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic == "target mode `compile` forbids target.rustc"));
}

#[test]
fn manifest_shape_requires_provenance_for_artifact_mode() {
    let mut diagnostics = Vec::new();
    validate_manifest_shape(
        &manifest(SourceKind::Artifact, TargetMode::Artifact),
        &mut diagnostics,
    );

    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic == "target mode `artifact` requires target.triple"));
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic == "target mode `artifact` requires target.rustc"));
}

#[test]
fn manifest_shape_rejects_bindings_for_generated_policy() {
    let mut manifest = manifest(SourceKind::Artifact, TargetMode::Artifact);
    manifest.bindings.push(Binding {
        source_module: "example".to_owned(),
        source_symbol: "VALUE".to_owned(),
        target: "example::VALUE".to_owned(),
    });
    let mut diagnostics = Vec::new();
    validate_manifest_shape(&manifest, &mut diagnostics);

    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic == "binding policy `generated` forbids [[bindings]] rows"));
}

#[test]
fn manifest_shape_requires_bindings_for_manifest_policy() {
    let mut manifest = manifest(SourceKind::Artifact, TargetMode::Artifact);
    manifest.target.binding_policy = BindingPolicy::Manifest;
    let mut diagnostics = Vec::new();
    validate_manifest_shape(&manifest, &mut diagnostics);

    assert!(diagnostics.iter().any(|diagnostic| diagnostic
        == "binding policy `manifest` requires at least one [[bindings]] row"));
}
