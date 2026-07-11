use super::*;

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
