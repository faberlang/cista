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

#[cfg(unix)]
#[test]
fn remote_archive_rejects_package_symlinks() {
    use std::os::unix::fs::symlink;

    let root = temp_root().join("remote-archive-symlink");
    fs::create_dir_all(&root).expect("create package root");
    fs::write(root.join("payload"), "inside").expect("write package payload");
    symlink("payload", root.join("alias")).expect("create package symlink");

    let error = archive_directory(&root).expect_err("package symlink should fail closed");

    assert!(error.contains("unsupported symlink"), "{error}");
    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn remote_archive_rejects_link_entries() {
    let mut archive = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_size(0);
    header.set_mode(0o777);
    header.set_cksum();
    archive
        .append_link(&mut header, "alias", "payload")
        .expect("append archive symlink");
    let bytes = archive.into_inner().expect("finish archive");
    let destination = temp_root().join("link-entry");
    fs::create_dir_all(&destination).expect("create destination");

    let error = unpack_archive(&bytes, &destination).expect_err("link entry should fail closed");

    assert!(error.contains("unsupported entry alias"), "{error}");
    assert!(!destination.join("alias").exists());
    fs::remove_dir_all(destination).expect("cleanup destination");
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
fn mismatched_local_registry_package_preserves_cached_package() {
    let root = temp_root().join("mismatched-local-package");
    let registry_package = root.join("registry/tool/1.2.3");
    let cached_package = root.join("store/.cache/registry/tool/1.2.3");
    fs::create_dir_all(&registry_package).expect("create registry package");
    fs::create_dir_all(&cached_package).expect("create cached package");
    fs::write(
        registry_package.join("cista.toml"),
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
    fs::write(cached_package.join("payload"), "last good package").expect("seed cache");

    let error = fetch_to_cache(
        "tool@1.2.3",
        Some(&root.join("registry")),
        Some(&root.join("store")),
    )
    .expect_err("mismatched identity should fail closed");

    assert!(error.contains("declares `other@9.9.9`"));
    assert_eq!(
        fs::read_to_string(cached_package.join("payload")).expect("read preserved cache"),
        "last good package"
    );
    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn invalid_remote_package_preserves_cached_package() {
    let root = temp_root().join("invalid-remote-package");
    let source = root.join("source");
    let destination = root.join("cached");
    fs::create_dir_all(&source).expect("create source");
    fs::create_dir_all(&destination).expect("create cached package");
    fs::write(
        source.join("cista.toml"),
        r#"[source]
package = "tool"
version = "1.2.3"
faber_min = "0.38.0"
kind = "source"
interfaces = "missing-interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
"#,
    )
    .expect("write invalid manifest");
    fs::write(destination.join("payload"), "last good package").expect("seed cache");

    let archive = archive_directory(&source).expect("archive invalid replacement");
    let error = install_remote_archive(&archive, &destination, "tool", "1.2.3")
        .expect_err("structurally invalid package should fail closed");

    assert!(error.contains("source.interfaces"), "{error}");
    assert_eq!(
        fs::read_to_string(destination.join("payload")).expect("read preserved cache"),
        "last good package"
    );
    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn invalid_local_registry_package_preserves_cached_package() {
    let root = temp_root().join("invalid-local-package");
    let registry_package = root.join("registry/tool/1.2.3");
    let cached_package = root.join("store/.cache/registry/tool/1.2.3");
    fs::create_dir_all(&registry_package).expect("create registry package");
    fs::create_dir_all(&cached_package).expect("create cached package");
    fs::write(
        registry_package.join("cista.toml"),
        r#"[source]
package = "tool"
version = "1.2.3"
faber_min = "0.38.0"
kind = "source"
interfaces = "missing-interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
"#,
    )
    .expect("write invalid manifest");
    fs::write(cached_package.join("payload"), "last good package").expect("seed cache");

    let error = fetch_to_cache(
        "tool@1.2.3",
        Some(&root.join("registry")),
        Some(&root.join("store")),
    )
    .expect_err("structurally invalid package should fail closed");

    assert!(error.contains("source.interfaces"), "{error}");
    assert_eq!(
        fs::read_to_string(cached_package.join("payload")).expect("read preserved cache"),
        "last good package"
    );
    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn local_registry_meta_dependency_paths_are_not_cached_as_trusted() {
    let root = temp_root().join("invalid-local-meta");
    let registry_package = root.join("registry/coreutils/1.0.0");
    let cached_package = root.join("store/.cache/registry/coreutils/1.0.0");
    fs::create_dir_all(&registry_package).expect("create registry package");
    fs::create_dir_all(&cached_package).expect("create cached package");
    fs::write(
        registry_package.join("cista.toml"),
        r#"[source]
package = "coreutils"
version = "1.0.0"
role = "meta"

[[dependencies]]
package = "true"
version = "1.0.0"
path = "../true"
"#,
    )
    .expect("write invalid meta manifest");
    fs::write(cached_package.join("payload"), "last good meta").expect("seed cache");

    let error = fetch_to_cache(
        "coreutils@1.0.0",
        Some(&root.join("registry")),
        Some(&root.join("store")),
    )
    .expect_err("meta dependency paths should fail closed for cache");

    assert!(
        error.contains("must not carry a source-relative path"),
        "{error}"
    );
    assert_eq!(
        fs::read_to_string(cached_package.join("payload")).expect("read preserved cache"),
        "last good meta"
    );
    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[cfg(unix)]
#[test]
fn local_registry_package_symlink_cannot_escape_registry() {
    use std::os::unix::fs::symlink;

    let root = temp_root().join("registry-package-symlink");
    let registry = root.join("registry");
    let external = root.join("external");
    let package_parent = registry.join("tool");
    fs::create_dir_all(&package_parent).expect("create registry package parent");
    fs::create_dir_all(&external).expect("create external package");
    fs::write(
        external.join("cista.toml"),
        "[source]\npackage = \"tool\"\nversion = \"1.2.3\"\n",
    )
    .expect("write external manifest");
    symlink(&external, package_parent.join("1.2.3")).expect("link escaped package");

    let error = fetch_to_cache("tool@1.2.3", Some(&registry), Some(&root.join("store")))
        .expect_err("escaped registry package should fail closed");

    assert!(error.contains("resolves outside registry"));
    assert!(!root.join("store/.cache/registry/tool/1.2.3").exists());
    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[cfg(unix)]
#[test]
fn local_registry_publish_cannot_follow_package_symlink_outside_registry() {
    use std::os::unix::fs::symlink;

    let root = temp_root().join("registry-publish-symlink");
    let source = root.join("source");
    let registry = root.join("registry");
    let external = root.join("external");
    fs::create_dir_all(source.join("interfaces")).expect("create source interfaces");
    fs::create_dir_all(&registry).expect("create registry");
    fs::create_dir_all(&external).expect("create external directory");
    fs::write(
        source.join("cista.toml"),
        r#"[source]
package = "tool"
version = "1.2.3"
faber_min = "0.38.0"
kind = "source"
interfaces = "interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
"#,
    )
    .expect("write package manifest");
    symlink(&external, registry.join("tool")).expect("link escaped package name");

    let error = publish(&source, Path::new("cista.toml"), Some(&registry))
        .expect_err("escaped registry destination should fail closed");

    assert!(error.contains("resolves outside registry"));
    assert!(!external.join("1.2.3").exists());
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

#[test]
fn publish_preserves_existing_empty_package_version() {
    let root = temp_root().join("immutable-empty-package");
    let source = root.join("source");
    let registry = root.join("registry");
    let destination = registry.join("tool/1.2.3");
    fs::create_dir_all(source.join("interfaces")).expect("create source interfaces");
    fs::create_dir_all(&destination).expect("reserve package version");
    fs::write(
        source.join("cista.toml"),
        r#"[source]
package = "tool"
version = "1.2.3"
faber_min = "0.38.0"
kind = "source"
interfaces = "interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
"#,
    )
    .expect("write package manifest");

    let error = publish(&source, Path::new("cista.toml"), Some(&registry))
        .expect_err("reserved package version should remain immutable");

    assert!(error.contains("already exists and is immutable"));
    assert!(fs::read_dir(&destination)
        .expect("read reserved package version")
        .next()
        .is_none());
    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn publish_rejects_registry_inside_package() {
    let root = temp_root().join("registry-inside-package");
    let source = root.join("source");
    let registry = source.join("registry");
    fs::create_dir_all(source.join("interfaces")).expect("create source interfaces");
    fs::write(
        source.join("cista.toml"),
        r#"[source]
package = "tool"
version = "1.2.3"
faber_min = "0.38.0"
kind = "source"
interfaces = "interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
"#,
    )
    .expect("write package manifest");
    assert!(!registry.exists(), "registry must start absent");

    let error = publish(&source, Path::new("cista.toml"), Some(&registry))
        .expect_err("registry inside package should fail closed");

    assert!(error.contains("cannot be inside published package"));
    assert!(!registry.exists());
    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn publish_rejects_destination_inside_package() {
    let root = temp_root().join("destination-inside-package");
    let registry = root.join("registry");
    let source = registry.join("tool");
    fs::create_dir_all(source.join("interfaces")).expect("create source interfaces");
    fs::write(
        source.join("cista.toml"),
        r#"[source]
package = "tool"
version = "1.2.3"
faber_min = "0.38.0"
kind = "source"
interfaces = "interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
"#,
    )
    .expect("write package manifest");

    let error = publish(&source, Path::new("cista.toml"), Some(&registry))
        .expect_err("destination inside package should fail closed");

    assert!(error.contains("destination"));
    assert!(error.contains("cannot be inside published package"));
    assert!(!source.join("1.2.3").exists());
    fs::remove_dir_all(root).expect("cleanup temp root");
}
