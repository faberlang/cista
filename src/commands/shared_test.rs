use super::*;
use crate::manifest::{Binding, PackageRole, SourceSection, TargetSection};
use std::fs;

fn temp_root(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "cista-shared-{name}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp root");
    path
}

fn buildable_manifest() -> CistaManifest {
    let mut manifest = manifest(SourceKind::Source, TargetMode::Compile);
    manifest.target.artifact = None;
    manifest.target.source = Some(PathBuf::from("target"));
    manifest.target.compile = Some(crate::manifest::CompileSection {
        emit: "library".to_owned(),
        crate_type: "rlib".to_owned(),
        edition: "2021".to_owned(),
    });
    manifest
}

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

#[test]
fn source_version_must_not_collide_with_transaction_directory_suffixes() {
    for version in ["1.0.0.incoming-123-1", "1.0.0.replaced-123-2"] {
        let mut manifest = buildable_manifest();
        manifest.source.version = version.to_owned();
        let mut diagnostics = Vec::new();

        validate_manifest_shape(&manifest, &mut diagnostics);

        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic
                .contains("collides with Cista install transaction directory namespace")),
            "missing transaction namespace diagnostic for {version}: {diagnostics:?}"
        );
        let identity_error = validate_identity("example", version)
            .expect_err("identity validation must reject transaction-like versions");
        assert!(
            identity_error.iter().any(|diagnostic| diagnostic
                .contains("collides with Cista install transaction directory namespace")),
            "missing identity diagnostic for {version}: {identity_error:?}"
        );
    }
}

#[test]
fn source_identity_segments_must_not_contain_at_signs() {
    for (field, package, version) in [
        ("source.package", "foo@bar", "1.0.0"),
        ("source.version", "foo", "1.0@0"),
    ] {
        let invalid_value = if field == "source.package" {
            package
        } else {
            version
        };
        let expected =
            format!("{field} `{invalid_value}` is not a valid package store path segment");
        let mut manifest = buildable_manifest();
        manifest.source.package = package.to_owned();
        manifest.source.version = version.to_owned();
        let mut diagnostics = Vec::new();

        validate_manifest_shape(&manifest, &mut diagnostics);

        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic == &expected),
            "missing @ diagnostic for {field}: {diagnostics:?}"
        );
        let identity_error = validate_identity(package, version)
            .expect_err("identity validation must reject @ in package identity segments");
        assert!(
            identity_error
                .iter()
                .any(|diagnostic| diagnostic == &expected),
            "missing @ identity diagnostic for {field}: {identity_error:?}"
        );
    }
}

#[test]
fn package_manifest_paths_must_be_relative_and_contained() {
    let root = temp_root("manifest-path-boundary");
    let package = root.join("package");
    let external = root.join("external");
    fs::create_dir_all(package.join("interfaces")).expect("create interfaces");
    fs::create_dir_all(package.join("target")).expect("create target source");
    fs::create_dir_all(&external).expect("create external root");

    let mut manifest = buildable_manifest();
    manifest.source.interfaces = external.join("interfaces");
    manifest.source.sources = Some(external.join("sources"));
    manifest.target.source = Some(external.join("target"));
    manifest.target.artifact = Some(external.join("artifact"));
    fs::write(
        package.join("cista.toml"),
        toml::to_string_pretty(&manifest).expect("serialize malicious manifest"),
    )
    .expect("write malicious manifest");

    let diagnostics = match validate_package(&package, Path::new("cista.toml"), None, false) {
        Ok(_) => panic!("absolute package paths must be rejected"),
        Err(diagnostics) => diagnostics,
    };
    for field in [
        "source.interfaces",
        "source.sources",
        "target.source",
        "target.artifact",
    ] {
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.contains(&format!("{field} path must be relative"))
            }),
            "missing boundary diagnostic for {field}: {diagnostics:?}"
        );
    }
    let parent_escape = resolve_package_path(&package, "target.source", Path::new("../external"))
        .expect_err("parent-escaping package path must be rejected");
    assert!(
        parent_escape.contains("must be normalized"),
        "{parent_escape}"
    );

    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[cfg(unix)]
#[test]
fn package_manifest_paths_must_resolve_symlinks_inside_package_root() {
    use std::os::unix::fs::symlink;

    let root = temp_root("manifest-symlink-boundary");
    let package = root.join("package");
    let external = root.join("external-target");
    fs::create_dir_all(package.join("interfaces")).expect("create interfaces");
    fs::create_dir_all(&external).expect("create external target");
    symlink(&external, package.join("target")).expect("create escaping target symlink");

    let manifest = buildable_manifest();
    fs::write(
        package.join("cista.toml"),
        toml::to_string_pretty(&manifest).expect("serialize symlink manifest"),
    )
    .expect("write symlink manifest");

    let diagnostics = match validate_package(&package, Path::new("cista.toml"), None, false) {
        Ok(_) => panic!("escaping symlink must be rejected"),
        Err(diagnostics) => diagnostics,
    };
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic
                .contains("target.source path resolves outside package root")),
        "missing symlink boundary diagnostic: {diagnostics:?}"
    );

    fs::remove_dir_all(root).expect("cleanup temp root");
}
