use super::*;
use crate::manifest::{PackageRole, SourceSection, TargetSection};

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
