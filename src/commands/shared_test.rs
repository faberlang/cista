use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::manifest::{
    Binding, BindingPolicy, PackageRole, SourceKind, SourceSection, TargetMode, TargetSection,
};
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

    let Err(diagnostics) = validate_package(&package, Path::new("cista.toml"), None, false) else {
        panic!("absolute package paths must be rejected")
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

    let Err(diagnostics) = validate_package(&package, Path::new("cista.toml"), None, false) else {
        panic!("escaping symlink must be rejected")
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

// --- verify_build=true ---

#[test]
fn validate_package_with_verify_build_rejects_non_rust_language() {
    let root = temp_root("verify-build-non-rust");
    let package = root.join("package");
    fs::create_dir_all(package.join("interfaces")).expect("create interfaces");
    fs::create_dir_all(package.join("target")).expect("create target source");

    let mut manifest = buildable_manifest();
    manifest.target.language = "python".to_owned();
    fs::write(
        package.join("cista.toml"),
        toml::to_string_pretty(&manifest).expect("serialize manifest"),
    )
    .expect("write manifest");

    let result = validate_package(&package, Path::new("cista.toml"), None, true);
    let Err(diagnostics) = result else {
        panic!("non-rust language must fail with verify_build=true")
    };
    assert!(diagnostics
        .iter()
        .any(|d| d.contains("only implemented for target.language")));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn validate_package_with_verify_build_skips_cargo_for_interfaces_only() {
    let root = temp_root("verify-build-interfaces-only");
    let package = root.join("package");
    fs::create_dir_all(package.join("interfaces")).expect("create interfaces");

    let mut manifest = buildable_manifest();
    manifest.target.source = None;
    manifest.target.compile = None;
    manifest.target.binding_policy = BindingPolicy::Generated;
    fs::write(
        package.join("cista.toml"),
        toml::to_string_pretty(&manifest).expect("serialize manifest"),
    )
    .expect("write manifest");

    let result = validate_package(&package, Path::new("cista.toml"), None, true);
    let Err(diagnostics) = result else {
        panic!("interfaces-only must fail with verify_build=true")
    };
    assert!(diagnostics.iter().any(|d| d.contains("nothing to check")));
    fs::remove_dir_all(root).expect("cleanup");
}

// TEST-BUG: The happy path for verify_target_build requires a real `cargo`
// toolchain. This test is omitted because `validate_target_paths` (called
// before verify_target_build) already requires Cargo.toml when target.source
// is set and language is rust, so the verify_build=true code path can only
// be reached when a Cargo.toml exists — which would trigger a real `cargo check`.

#[test]
fn validate_package_with_verify_build_is_disabled_by_default() {
    let root = temp_root("verify-build-default");
    let package = root.join("package");
    fs::create_dir_all(package.join("interfaces")).expect("create interfaces");

    let mut manifest = buildable_manifest();
    manifest.target.source = None;
    manifest.target.compile = None;
    manifest.target.binding_policy = BindingPolicy::Generated;
    fs::write(
        package.join("cista.toml"),
        toml::to_string_pretty(&manifest).expect("serialize manifest"),
    )
    .expect("write manifest");

    let result = validate_package(&package, Path::new("cista.toml"), None, false);
    assert!(
        result.is_ok(),
        "interfaces-only package should validate without verify_build"
    );
    fs::remove_dir_all(root).expect("cleanup");
}

// --- resolve_meta_dependency_path ---

#[test]
fn resolve_meta_dependency_rejects_empty_path() {
    let root = temp_root("meta-empty-path");
    fs::create_dir_all(&root).expect("create meta root");

    let error = resolve_meta_dependency_path(&root, "dependency", Path::new(""))
        .expect_err("empty path must be rejected");
    assert!(error.contains("must not be empty"));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn resolve_meta_dependency_rejects_absolute_path() {
    let root = temp_root("meta-absolute-path");
    fs::create_dir_all(&root).expect("create meta root");

    let error = resolve_meta_dependency_path(&root, "dependency", Path::new("/etc/passwd"))
        .expect_err("absolute path must be rejected");
    assert!(error.contains("must be relative"));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn resolve_meta_dependency_resolves_sibling_path() {
    let root = temp_root("meta-sibling");
    let meta_root = root.join("meta-package");
    let dep_root = root.join("dep-package");
    fs::create_dir_all(&meta_root).expect("create meta root");
    fs::create_dir_all(&dep_root).expect("create dep root");

    let resolved =
        resolve_meta_dependency_path(&meta_root, "dependency", Path::new("../dep-package"))
            .expect("sibling path should resolve");
    assert!(resolved.ends_with("dep-package"));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn resolve_meta_dependency_rejects_traversal_beyond_collection() {
    let root = temp_root("meta-escape");
    let collection = root.join("collection");
    let meta_root = collection.join("meta-package");
    let outside = root.join("outside");
    fs::create_dir_all(&meta_root).expect("create meta root");
    fs::create_dir_all(&outside).expect("create outside dir");

    let error = resolve_meta_dependency_path(&meta_root, "dependency", Path::new("../../outside"))
        .expect_err("traversal beyond collection must be rejected");
    assert!(error.contains("resolves outside package collection"));
    fs::remove_dir_all(root).expect("cleanup");
}

// --- validate_interfaces ---

#[test]
fn validate_interfaces_detects_missing_interface_directory() {
    let root = temp_root("missing-interface-dir");
    let package = root.join("package");
    fs::create_dir_all(&package).expect("create package root");

    let mut manifest = buildable_manifest();
    manifest.bindings.push(crate::manifest::Binding {
        source_module: "example".to_owned(),
        source_symbol: "VALUE".to_owned(),
        target: "example::VALUE".to_owned(),
    });

    let mut diagnostics = Vec::new();
    let symbols = validate_interfaces(
        &package,
        &package.join("interfaces"),
        &manifest,
        &mut diagnostics,
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| d.contains("does not point to a directory")),
        "missing interface dir should produce diagnostic: {diagnostics:?}"
    );
    assert!(symbols.is_empty(), "no symbols should be extracted");
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn validate_interfaces_extracts_symbols_from_fab_files() {
    let root = temp_root("extract-symbols");
    let package = root.join("package");
    let interfaces = package.join("interfaces");
    fs::create_dir_all(&interfaces).expect("create interfaces dir");

    fs::write(
        interfaces.join("example.fab"),
        "functio run(a: i32) -> i32\nfunctio setup()\n",
    )
    .expect("write interface file");

    let mut manifest = buildable_manifest();
    manifest.bindings.push(crate::manifest::Binding {
        source_module: "example".to_owned(),
        source_symbol: "run".to_owned(),
        target: "example::run".to_owned(),
    });

    // Canonicalize package root so starts_with checks work when
    // /tmp or /var is a symlink (macOS: /var -> /private/var).
    let canonical_package = package.canonicalize().expect("canonicalize package root");
    let mut diagnostics = Vec::new();
    let symbols = validate_interfaces(&canonical_package, &interfaces, &manifest, &mut diagnostics);
    assert!(
        diagnostics.is_empty(),
        "no diagnostics expected: {diagnostics:?}"
    );
    let example_symbols = symbols
        .get("example")
        .expect("example module should have symbols");
    assert!(
        example_symbols.contains("run"),
        "should contain 'run' symbol"
    );
    fs::remove_dir_all(root).expect("cleanup");
}

// --- validate_target_paths ---

#[test]
fn validate_target_paths_rejects_missing_target_source() {
    let root = temp_root("missing-target-source");
    let package = root.join("package");
    fs::create_dir_all(package.join("interfaces")).expect("create interfaces");

    let manifest = buildable_manifest();
    let paths = PackagePaths {
        interfaces: Some(package.join("interfaces")),
        target_source: Some(package.join("nonexistent")),
        artifact: None,
    };

    let mut diagnostics = Vec::new();
    validate_target_paths(&paths, &manifest, &mut diagnostics);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.contains("does not point to a directory")),
        "missing target source should produce diagnostic: {diagnostics:?}"
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn validate_target_paths_rejects_missing_artifact_file() {
    let root = temp_root("missing-artifact");
    let package = root.join("package");
    fs::create_dir_all(package.join("interfaces")).expect("create interfaces");

    let manifest = {
        let mut m = buildable_manifest();
        m.target.mode = TargetMode::Artifact;
        m.source.kind = SourceKind::Artifact;
        m.target.source = None;
        m.target.compile = None;
        m.target.triple = Some("test-triple".to_owned());
        m.target.rustc = Some("rustc 1.88".to_owned());
        m
    };
    let paths = PackagePaths {
        interfaces: Some(package.join("interfaces")),
        target_source: None,
        artifact: Some(package.join("nonexistent.rlib")),
    };

    let mut diagnostics = Vec::new();
    validate_target_paths(&paths, &manifest, &mut diagnostics);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.contains("does not point to a file")),
        "missing artifact should produce diagnostic: {diagnostics:?}"
    );
    fs::remove_dir_all(root).expect("cleanup");
}

// --- validate_bindings ---

#[test]
fn validate_bindings_reports_missing_symbol() {
    let mut symbols = BTreeMap::new();
    let mut module_symbols = BTreeSet::new();
    module_symbols.insert("run".to_owned());
    symbols.insert("example".to_owned(), module_symbols);

    let manifest = {
        let mut m = buildable_manifest();
        m.bindings.push(crate::manifest::Binding {
            source_module: "example".to_owned(),
            source_symbol: "nonexistent".to_owned(),
            target: "example::nonexistent".to_owned(),
        });
        m
    };

    let mut diagnostics = Vec::new();
    validate_bindings(&manifest, &symbols, &mut diagnostics);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.contains("not found in module")),
        "missing symbol should produce diagnostic: {diagnostics:?}"
    );
}
