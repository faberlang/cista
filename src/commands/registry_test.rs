use super::*;
use crate::cli::{CistaCli, CistaCommand};
use clap::Parser;
use std::fs;

fn temp_root() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("cista-registry-{}-{nanos}", std::process::id()))
}

#[test]
fn archive_round_trip_preserves_package_tree() {
    let root = temp_root();
    let source = root.join("source");
    let destination = root.join("destination");
    fs::create_dir_all(source.join("interfaces")).unwrap();
    fs::write(source.join("cista.toml"), "[source]\npackage = \"tool\"\n").unwrap();
    fs::write(
        source.join("interfaces/tool.fab"),
        "functio main() → nihil\n",
    )
    .unwrap();

    let archive = archive_directory(&source).unwrap();
    fs::create_dir_all(&destination).unwrap();
    unpack_archive(&archive, &destination).unwrap();
    assert!(destination.join("cista.toml").is_file());
    assert!(destination.join("interfaces/tool.fab").is_file());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_remote_archive_preserves_cached_package() {
    let root = temp_root().join("invalid-remote-archive");
    let source = root.join("source");
    let destination = root.join("cached");
    fs::create_dir_all(&source).expect("create source");
    fs::create_dir_all(&destination).expect("create cached package");
    fs::write(source.join("payload"), "replacement without manifest")
        .expect("write invalid replacement");
    fs::write(destination.join("payload"), "last good package").expect("seed cache");

    let archive = archive_directory(&source).expect("archive invalid replacement");
    let error = install_remote_archive(&archive, &destination, "tool", "1.2.3")
        .expect_err("missing manifest should fail closed");

    assert!(error.contains("archive has no cista.toml"));
    assert_eq!(
        fs::read_to_string(destination.join("payload")).expect("read preserved cache"),
        "last good package"
    );
    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn mismatched_remote_archive_preserves_cached_package() {
    let root = temp_root().join("mismatched-remote-archive");
    let source = root.join("source");
    let destination = root.join("cached");
    fs::create_dir_all(&source).expect("create source");
    fs::create_dir_all(&destination).expect("create cached package");
    fs::write(
        source.join("cista.toml"),
        r#"[source]
package = "other"
version = "9.9.9"
faber_min = "0.38.0"
kind = "source"
interfaces = "interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
"#,
    )
    .expect("write mismatched manifest");
    fs::write(destination.join("payload"), "last good package").expect("seed cache");

    let archive = archive_directory(&source).expect("archive mismatched replacement");
    let error = install_remote_archive(&archive, &destination, "tool", "1.2.3")
        .expect_err("mismatched identity should fail closed");

    assert!(error.contains("archive declares `other@9.9.9`"));
    assert_eq!(
        fs::read_to_string(destination.join("payload")).expect("read preserved cache"),
        "last good package"
    );
    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn cli_routes_remote_registry_without_accepting_local_registry_too() {
    let cli = CistaCli::try_parse_from([
        "cista",
        "fetch",
        "tool@1.2.3",
        "--registry-url",
        "https://cista.dev",
    ])
    .unwrap();
    let CistaCommand::Fetch(args) = cli.command else {
        panic!("expected fetch command");
    };
    assert_eq!(args.registry_url.as_deref(), Some("https://cista.dev"));
    assert!(CistaCli::try_parse_from([
        "cista",
        "fetch",
        "tool@1.2.3",
        "--registry-url",
        "https://cista.dev",
        "--registry",
        "/tmp/registry",
    ])
    .is_err());
}

#[test]
fn publish_and_fetch_exact_package_snapshot() {
    let root = temp_root();
    let source = root.join("source");
    let registry = root.join("registry");
    let store = root.join("store");
    fs::create_dir_all(source.join("interfaces")).expect("create interfaces");
    fs::create_dir_all(source.join("rust/src")).expect("create rust source");
    fs::write(
        source.join("interfaces/tool.fab"),
        "functio main() → nihil\n",
    )
    .expect("write interface");
    fs::write(
        source.join("cista.toml"),
        r#"[source]
package = "tool"
version = "1.2.3"
faber_min = "0.38.0"
kind = "source"
role = "bin"
interfaces = "interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
source = "rust"
crate = "tool"

[target.compile]
emit = "binary"
crate_type = "bin"
edition = "2021"
"#,
    )
    .expect("write cista manifest");
    fs::write(
        source.join("rust/Cargo.toml"),
        "[package]\nname = \"tool\"\nversion = \"1.2.3\"\nedition = \"2021\"\n",
    )
    .expect("write cargo manifest");
    fs::write(source.join("rust/src/main.rs"), "fn main() {}\n").expect("write rust source");

    publish(&source, Path::new("cista.toml"), Some(&registry)).expect("publish snapshot");
    fs::remove_dir_all(&source).expect("remove original source");
    let fetched =
        fetch_to_cache("tool@1.2.3", Some(&registry), Some(&store)).expect("fetch exact package");
    assert!(fetched.join("cista.toml").is_file());
    assert!(fetched.join("rust/src/main.rs").is_file());
    assert!(publish(&fetched, Path::new("cista.toml"), Some(&registry)).is_err());
    assert!(fetch_to_cache("tool", Some(&registry), Some(&store)).is_err());
    assert!(fetch_to_cache("../tool@1.2.3", Some(&registry), Some(&store)).is_err());

    fs::remove_dir_all(root).expect("cleanup temp root");
}
