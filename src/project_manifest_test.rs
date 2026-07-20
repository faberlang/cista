use std::fs;

use super::*;

fn temp_path(label: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "cista-project-manifest-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path.join("faber.toml")
}

#[test]
fn require_exact_dependency_succeeds_on_match() {
    let manifest = ProjectManifest {
        package: ProjectPackage {
            name: "demo".to_owned(),
            version: Some("0.1.0".to_owned()),
            edition: None,
        },
        dependencies: [("norma".to_owned(), "0.1.0".to_owned())].into(),
        paths: None,
        build: None,
        reader: None,
    };

    let result = require_exact_dependency(&manifest, "norma", "0.1.0");
    assert!(
        result.is_ok(),
        "matching version should succeed: {result:?}"
    );
}

#[test]
fn require_exact_dependency_rejects_version_mismatch() {
    let manifest = ProjectManifest {
        package: ProjectPackage {
            name: "demo".to_owned(),
            version: Some("0.1.0".to_owned()),
            edition: None,
        },
        dependencies: [("norma".to_owned(), "0.1.0".to_owned())].into(),
        paths: None,
        build: None,
        reader: None,
    };

    let error = require_exact_dependency(&manifest, "norma", "0.2.0")
        .expect_err("version mismatch must be rejected");
    assert!(error.contains("declares"));
    assert!(error.contains("0.1.0"));
    assert!(error.contains("0.2.0"));
}

#[test]
fn require_exact_dependency_rejects_missing_dependency() {
    let manifest = ProjectManifest {
        package: ProjectPackage {
            name: "demo".to_owned(),
            version: Some("0.1.0".to_owned()),
            edition: None,
        },
        dependencies: [("norma".to_owned(), "0.1.0".to_owned())].into(),
        paths: None,
        build: None,
        reader: None,
    };

    let error = require_exact_dependency(&manifest, "missing-dep", "0.1.0")
        .expect_err("missing dependency must be rejected");
    assert!(error.contains("not declared"));
    assert!(error.contains("missing-dep"));
}

#[test]
fn read_project_manifest_parses_strict_valid_manifest() {
    let path = temp_path("strict-valid");
    fs::write(
        &path,
        r#"[package]
name = "demo"
version = "0.1.0"

[dependencies]
norma = "0.1.0"
"#,
    )
    .expect("write manifest");

    let manifest = read_project_manifest(&path).expect("valid strict manifest must parse");
    assert_eq!(manifest.package.name, "demo");
    assert_eq!(
        manifest.dependencies.get("norma"),
        Some(&"0.1.0".to_owned())
    );
    fs::remove_dir_all(path.parent().unwrap()).expect("cleanup");
}

#[test]
fn read_project_manifest_falls_back_to_loose_parse_for_unknown_fields() {
    let path = temp_path("loose-fallback");
    fs::write(
        &path,
        r#"[package]
name = "demo"
version = "0.1.0"

future-section = true

[dependencies]
norma = "0.1.0"
"#,
    )
    .expect("write manifest");

    let manifest = read_project_manifest(&path)
        .expect("manifest with unknown top-level keys should fall back to loose parse");
    assert_eq!(manifest.package.name, "demo");
    assert_eq!(
        manifest.dependencies.get("norma"),
        Some(&"0.1.0".to_owned())
    );
    fs::remove_dir_all(path.parent().unwrap()).expect("cleanup");
}

#[test]
fn read_project_manifest_rejects_missing_package_section() {
    let path = temp_path("missing-package");
    fs::write(
        &path,
        r#"
[dependencies]
norma = "0.1.0"
"#,
    )
    .expect("write manifest");

    let error = read_project_manifest(&path).expect_err("missing [package] must be rejected");
    assert!(error.contains("missing [package]"));
    fs::remove_dir_all(path.parent().unwrap()).expect("cleanup");
}

#[test]
fn read_project_manifest_rejects_missing_package_name() {
    let path = temp_path("missing-name");
    fs::write(
        &path,
        r#"
[package]
version = "0.1.0"

[dependencies]
norma = "0.1.0"
"#,
    )
    .expect("write manifest");

    let error = read_project_manifest(&path).expect_err("missing package.name must be rejected");
    assert!(error.contains("package.name is required"));
    fs::remove_dir_all(path.parent().unwrap()).expect("cleanup");
}

#[test]
fn loose_parse_rejects_non_table_dependencies() {
    let error = parse_dependencies_loose(
        r#"
dependencies = "norma"

[package]
name = "demo"
"#,
        Path::new("faber.toml"),
    )
    .expect_err("dependencies must be a table");

    assert_eq!(error, "faber.toml [dependencies] must be a table");
}

#[test]
fn loose_parse_accepts_inline_dependency_versions() {
    let manifest = parse_dependencies_loose(
        r#"
future-setting = true

[package]
name = "demo"

[dependencies]
norma = { version = "0.1.0" }
"#,
        Path::new("faber.toml"),
    )
    .expect("parse forward-compatible manifest");

    assert_eq!(
        manifest.dependencies.get("norma"),
        Some(&"0.1.0".to_owned())
    );
}
