use std::fs;
use std::path::PathBuf;

use super::*;

fn temp_dir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "cista-inspect-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

#[test]
fn inspect_path_rejects_missing_manifest() {
    let root = temp_dir("missing-manifest");

    let error = inspect_path(&root).expect_err("missing manifest must be rejected");
    assert!(error.iter().any(|d| d.contains("no cista.toml")));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn inspect_path_succeeds_with_valid_manifest() {
    let root = temp_dir("valid-manifest");
    fs::create_dir_all(root.join("interfaces")).expect("create interfaces");
    fs::create_dir_all(root.join("target")).expect("create target");
    fs::write(
        root.join("cista.toml"),
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
"#,
    )
    .expect("write manifest");

    let result = inspect_path(&root);
    assert!(result.is_ok(), "inspect path should succeed: {result:?}");
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn inspect_path_rejects_invalid_manifest() {
    let root = temp_dir("invalid-manifest");
    fs::write(
        root.join("cista.toml"),
        "this is not valid TOML {{{",
    )
    .expect("write invalid manifest");

    let error = inspect_path(&root).expect_err("invalid manifest must be rejected");
    assert!(error.iter().any(|d| d.contains("failed to parse manifest")));
    fs::remove_dir_all(root).expect("cleanup");
}
