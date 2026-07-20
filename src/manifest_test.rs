use std::fs;

use super::*;

fn temp_dir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "cista-manifest-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn write_manifest(path: &Path, content: &str) {
    fs::write(path, content).expect("write manifest");
}

fn valid_compile_toml() -> String {
    r#"[source]
package = "example"
version = "0.1.0"
faber_min = "0.38.0"
kind = "source"
interfaces = "interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
source = "target"
crate = "example"

[target.compile]
emit = "library"
crate_type = "rlib"
edition = "2021"
"#
    .to_owned()
}

fn valid_meta_toml() -> String {
    r#"[source]
package = "meta-example"
version = "0.1.0"
role = "meta"
"#
    .to_owned()
}

// --- manifest_path ---

#[test]
fn manifest_path_defaults_to_cista_toml_when_no_name_given() {
    let root = temp_dir("path-default");
    let path = manifest_path(&root, None);
    assert_eq!(path, root.join("cista.toml"));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn manifest_path_uses_custom_name_when_given() {
    let root = temp_dir("path-custom");
    let path = manifest_path(&root, Some(std::path::Path::new("custom.toml")));
    assert_eq!(path, root.join("custom.toml"));
    fs::remove_dir_all(root).expect("cleanup");
}

// --- read_manifest ---

#[test]
fn read_manifest_parses_valid_toml() {
    let root = temp_dir("read-valid");
    let path = root.join("cista.toml");
    write_manifest(&path, &valid_compile_toml());

    let manifest = read_manifest(&path).expect("valid manifest should parse");
    assert_eq!(manifest.source.package, "example");
    assert_eq!(manifest.source.version, "0.1.0");
    assert_eq!(manifest.target.language, "rust");
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn read_manifest_rejects_malformed_toml() {
    let root = temp_dir("read-malformed");
    let path = root.join("cista.toml");
    write_manifest(&path, "this is not valid toml {{{");

    let error = read_manifest(&path).expect_err("malformed TOML must be rejected");
    assert!(error.contains("failed to parse manifest"));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn read_manifest_rejects_unknown_fields() {
    let root = temp_dir("read-unknown");
    let path = root.join("cista.toml");
    write_manifest(
        &path,
        &format!("{}\nunknown_field = true\n", valid_compile_toml()),
    );

    let error = read_manifest(&path).expect_err("unknown fields must be rejected");
    assert!(error.contains("unknown field"));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn read_manifest_rejects_empty_file() {
    let root = temp_dir("read-empty");
    let path = root.join("cista.toml");
    write_manifest(&path, "");

    let error = read_manifest(&path).expect_err("empty manifest must be rejected");
    assert!(error.contains("failed to parse manifest"));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn read_manifest_rejects_missing_required_source_package() {
    let root = temp_dir("missing-package");
    let path = root.join("cista.toml");
    write_manifest(
        &path,
        r#"[source]
version = "0.1.0"
faber_min = "0.38.0"
kind = "source"
interfaces = "interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
source = "target"
crate = "example"

[target.compile]
emit = "library"
crate_type = "rlib"
edition = "2021"
"#,
    );

    let error = read_manifest(&path).expect_err("missing required field must be rejected");
    assert!(error.contains("missing field"));
    fs::remove_dir_all(root).expect("cleanup");
}

// --- read_meta_manifest ---

#[test]
fn read_meta_manifest_returns_some_for_meta_role() {
    let root = temp_dir("meta-some");
    let path = root.join("cista.toml");
    write_manifest(&path, &valid_meta_toml());

    let result = read_meta_manifest(&path).expect("meta manifest should parse");
    assert!(result.is_some());
    let meta = result.unwrap();
    assert_eq!(meta.source.package, "meta-example");
    assert_eq!(meta.source.version, "0.1.0");
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn read_meta_manifest_returns_none_for_regular_manifest() {
    let root = temp_dir("meta-none");
    let path = root.join("cista.toml");
    write_manifest(&path, &valid_compile_toml());

    let result = read_meta_manifest(&path).expect("regular manifest should parse as non-meta");
    assert!(result.is_none());
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn read_meta_manifest_rejects_malformed_toml() {
    let root = temp_dir("meta-malformed");
    let path = root.join("cista.toml");
    write_manifest(&path, "garbage {{{");

    let error = read_meta_manifest(&path).expect_err("malformed TOML must be rejected");
    assert!(error.contains("failed to parse manifest"));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn read_meta_manifest_rejects_meta_with_unknown_fields() {
    let root = temp_dir("meta-unknown");
    let path = root.join("cista.toml");
    write_manifest(
        &path,
        r#"[source]
package = "meta-example"
version = "0.1.0"
role = "meta"
unknown = true
"#,
    );

    let error = read_meta_manifest(&path).expect_err("meta with unknown fields must be rejected");
    assert!(error.contains("unknown field"));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn read_meta_manifest_parses_meta_with_dependencies() {
    let root = temp_dir("meta-deps");
    let path = root.join("cista.toml");
    write_manifest(
        &path,
        r#"[source]
package = "bundle"
version = "1.0.0"
role = "meta"

[[dependencies]]
package = "dep-a"
version = "0.1.0"
"#,
    );

    let result = read_meta_manifest(&path).expect("meta with dependencies should parse");
    assert!(result.is_some());
    let meta = result.unwrap();
    assert_eq!(meta.dependencies.len(), 1);
    assert_eq!(meta.dependencies[0].package, "dep-a");
    fs::remove_dir_all(root).expect("cleanup");
}
