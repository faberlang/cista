use super::*;
use crate::cli::{CistaCli, CistaCommand};
use clap::Parser;
use std::fs;
use std::io::Cursor;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn temp_root() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("cista-registry-{}-{nanos}", std::process::id()))
}

fn write_interfaces_only_registry_package(package: &Path, name: &str, version: &str) {
    fs::create_dir_all(package.join("interfaces")).expect("create package interfaces");
    fs::write(
        package.join("cista.toml"),
        format!(
            r#"[source]
package = "{name}"
version = "{version}"
faber_min = "0.38.0"
kind = "source"
interfaces = "interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
"#
        ),
    )
    .expect("write package manifest");
    fs::write(
        package.join("interfaces").join(format!("{name}.fab")),
        "functio lege() → nihil { redde nihil }\n",
    )
    .expect("write package interface");
}

/// Records the publish-time digest for a hand-built registry package.
fn seal(registry_package: &Path) {
    write_content_digest_record(registry_package).expect("record registry digest");
}

#[test]
fn fetch_to_cache_waits_for_store_mutation_lock() {
    let root = temp_root().join("fetch-cache-lock");
    let registry = root.join("registry");
    let registry_package = registry.join("tool/1.2.3");
    let store = root.join("store");
    write_interfaces_only_registry_package(&registry_package, "tool", "1.2.3");
    seal(&registry_package);

    let lock = shared::acquire_store_mutation_locks(&store, None).expect("hold store lock");
    let cache = store
        .canonicalize()
        .expect("canonicalize store root")
        .join(".cache/registry/tool/1.2.3");
    let (done_tx, done_rx) = mpsc::channel();
    let fetch_store = store.clone();
    let handle = thread::spawn(move || {
        let result = fetch_to_cache("tool@1.2.3", Some(&registry), Some(&fetch_store));
        done_tx.send(result).expect("send fetch result");
    });

    thread::sleep(Duration::from_millis(200));
    assert!(
        !cache.exists(),
        "fetch must not mutate cache while store mutation lock is held"
    );
    assert!(
        done_rx.try_recv().is_err(),
        "fetch should wait for the held store mutation lock"
    );

    drop(lock);
    let fetched = done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("fetch should finish after lock release")
        .expect("fetch should succeed");
    handle.join().expect("fetch thread should not panic");

    assert_eq!(fetched, cache);
    assert!(cache.join("cista.toml").is_file());
    fs::remove_dir_all(root).expect("cleanup temp root");
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

fn archive_with_mode(path: &str, mode: u32) -> Vec<u8> {
    let mut archive = tar::Builder::new(Vec::new());
    let payload = b"malicious payload";
    let mut header = tar::Header::new_gnu();
    header.set_size(payload.len() as u64);
    header.set_mode(mode);
    header.set_cksum();
    archive
        .append_data(&mut header, path, Cursor::new(payload))
        .expect("append archive entry");
    archive.into_inner().expect("finish archive")
}

#[test]
fn remote_archive_rejects_world_writable_file_mode_before_cache_write() {
    let root = temp_root().join("world-writable-remote-archive");
    let destination = root.join("cached");
    let archive = archive_with_mode("payload", 0o777);

    let error = install_remote_archive(&archive, &destination, "tool", "1.2.3")
        .expect_err("world-writable archive entry should fail closed");

    assert!(error.contains("dangerous mode 0o777"), "{error}");
    assert!(!destination.exists());
    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn remote_archive_rejects_setuid_world_writable_file_mode_before_cache_write() {
    let root = temp_root().join("setuid-world-writable-remote-archive");
    let destination = root.join("cached");
    let archive = archive_with_mode("payload", 0o4777);

    let error = install_remote_archive(&archive, &destination, "tool", "1.2.3")
        .expect_err("setuid archive entry should fail closed");

    assert!(error.contains("dangerous mode 0o4777"), "{error}");
    assert!(!destination.exists());
    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn registry_exact_identity_rejects_at_sign_inside_segments() {
    for package_id in ["foo@bar@1.0.0", "foo@1.0@0"] {
        let error = exact_identity(package_id)
            .expect_err("ambiguous @ inside identity segment must fail closed");
        assert!(
            error.contains("is not a valid package store path segment"),
            "{error}"
        );
    }
}

#[test]
fn registry_exact_identity_rejects_empty_string() {
    let error = exact_identity("").expect_err("empty identity must be rejected");
    assert!(
        error.contains("must use an exact name@version pin"),
        "{error}"
    );
}

#[test]
fn registry_exact_identity_rejects_missing_version() {
    let error = exact_identity("tool").expect_err("identity without version must be rejected");
    assert!(error.contains("version"), "{error}");
}

#[test]
fn registry_exact_identity_rejects_missing_package() {
    let error = exact_identity("@1.0.0").expect_err("identity without package must be rejected");
    assert!(error.contains("package"), "{error}");
}

#[test]
fn registry_archive_directory_rejects_nonexistent_path() {
    let root = tempfile::tempdir().expect("create temp root");
    let missing = root.path().join("nonexistent-archive");
    let error = archive_directory(&missing).expect_err("archiving a nonexistent path must fail");
    assert!(
        error.contains("No such file or directory") || error.contains("not found"),
        "{error}"
    );
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
    seal(&registry_package);
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
    seal(&registry_package);
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
    seal(&registry_package);
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
    assert!(
        CistaCli::try_parse_from([
            "cista",
            "fetch",
            "tool@1.2.3",
            "--registry-url",
            "https://cista.dev",
            "--registry",
            "/tmp/registry",
        ])
        .is_err()
    );
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

    // Hold references for sad-path tests.
    let registry_path = registry;
    let store_path = store;
    let fetched_path = fetched;
    assert!(
        publish(&fetched_path, Path::new("cista.toml"), Some(&registry_path)).is_err(),
        "publishing the fetched snapshot again must be rejected as immutable"
    );
    assert!(
        fetch_to_cache("tool", Some(&registry_path), Some(&store_path)).is_err(),
        "unversioned fetch must be rejected"
    );
    assert!(
        fetch_to_cache("../tool@1.2.3", Some(&registry_path), Some(&store_path)).is_err(),
        "path-traversal fetch must be rejected"
    );

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
    assert!(
        fs::read_dir(&destination)
            .expect("read reserved package version")
            .next()
            .is_none()
    );
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

fn write_minimal_publish_source(source: &Path) {
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
    fs::write(
        source.join("interfaces/tool.fab"),
        "functio lege() → nihil { redde nihil }\n",
    )
    .expect("write package interface");
}

#[test]
fn publish_records_content_digest_of_published_directory() {
    let root = temp_root().join("publish-digest-record");
    let source = root.join("source");
    let registry = root.join("registry");
    write_minimal_publish_source(&source);

    let destination =
        publish(&source, Path::new("cista.toml"), Some(&registry)).expect("publish package");

    let record = destination
        .parent()
        .expect("package directory")
        .join("1.2.3.content-sha256");
    let recorded = fs::read_to_string(&record).expect("read digest record");
    let expected = crate::faber_lock::staged_content_sha256(&destination).expect("digest");
    assert_eq!(recorded, expected);
    assert_eq!(recorded.len(), 64);

    let error = publish(&source, Path::new("cista.toml"), Some(&registry))
        .expect_err("republish must stay immutable");
    assert!(error.contains("already exists and is immutable"));
    assert_eq!(fs::read_to_string(&record).expect("record kept"), expected);
    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn publish_record_write_failure_leaves_no_published_directory() {
    let root = temp_root().join("publish-digest-record-failure");
    let source = root.join("source");
    let registry = root.join("registry");
    write_minimal_publish_source(&source);
    // A directory squatting on the record path makes the record write fail.
    let record = registry.join("tool/1.2.3.content-sha256");
    fs::create_dir_all(&record).expect("block record path");

    let error = publish(&source, Path::new("cista.toml"), Some(&registry))
        .expect_err("record write failure must fail publish");

    assert!(error.contains("content digest record"));
    assert!(!registry.join("tool/1.2.3").exists());
    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn fetch_installs_package_when_content_digest_record_matches() {
    let root = temp_root().join("fetch-digest-match");
    let source = root.join("source");
    let registry = root.join("registry");
    let store = root.join("store");
    write_minimal_publish_source(&source);
    publish(&source, Path::new("cista.toml"), Some(&registry)).expect("publish package");

    let fetched =
        fetch_to_cache("tool@1.2.3", Some(&registry), Some(&store)).expect("matching record");

    assert!(fetched.join("cista.toml").is_file());
    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn fetch_refuses_package_without_content_digest_record_before_cache_copy() {
    let root = temp_root().join("fetch-digest-absent");
    let registry_package = root.join("registry/tool/1.2.3");
    let store = root.join("store");
    write_interfaces_only_registry_package(&registry_package, "tool", "1.2.3");

    let error = fetch_to_cache("tool@1.2.3", Some(&root.join("registry")), Some(&store))
        .expect_err("absent record must be refused");

    assert!(error.contains("tool@1.2.3"), "{error}");
    assert!(error.contains("republish"), "{error}");
    assert!(!store.join(".cache/registry/tool/1.2.3").exists());
    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn fetch_refuses_package_changed_after_publish_before_cache_copy() {
    let root = temp_root().join("fetch-digest-swapped");
    let source = root.join("source");
    let registry = root.join("registry");
    let store = root.join("store");
    write_minimal_publish_source(&source);
    let published =
        publish(&source, Path::new("cista.toml"), Some(&registry)).expect("publish package");
    fs::write(
        published.join("interfaces/tool.fab"),
        "functio swapped() → nihil { redde nihil }\n",
    )
    .expect("swap published bytes");

    let error = fetch_to_cache("tool@1.2.3", Some(&registry), Some(&store))
        .expect_err("swapped bytes must be refused");

    assert!(error.contains("tool@1.2.3"), "{error}");
    assert!(error.contains("does not match"), "{error}");
    assert!(!store.join(".cache/registry/tool/1.2.3").exists());
    fs::remove_dir_all(root).expect("cleanup temp root");
}

fn write_incoming_archive_fixture(source: &Path, project: &Path) {
    fs::create_dir_all(source.join("interfaces")).expect("create interfaces");
    fs::create_dir_all(source.join("rust/src")).expect("create Rust source");
    fs::create_dir_all(project).expect("create project");
    fs::write(
        source.join("cista.toml"),
        r#"[source]
package = "gatedpkg"
version = "0.1.0"
faber_min = "0.38.0"
kind = "source"
interfaces = "interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
crate = "gatedpkg"
source = "rust"

[target.compile]
emit = "library"
crate_type = "rlib"
edition = "2021"
"#,
    )
    .expect("write cista manifest");
    fs::write(
        source.join("interfaces/gatedpkg.fab"),
        "functio lege() → nihil { redde nihil }\n",
    )
    .expect("write interface");
    fs::write(
        source.join("rust/Cargo.toml"),
        "[package]\nname = \"gatedpkg\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write Cargo manifest");
    fs::write(source.join("rust/src/lib.rs"), "pub fn gated() {}\n").expect("write Rust library");
    fs::write(
        project.join("faber.toml"),
        r#"[package]
name = "app"
version = "0.1.0"
edition = "2026"

[paths]
source = "src"
entry = "main.fab"

[dependencies]
gatedpkg = "0.1.0"
"#,
    )
    .expect("write project manifest");
}

#[test]
fn changed_incoming_archive_is_rejected_before_cargo() {
    for verify_target_build in [false, true] {
        let root = temp_root().join(format!("incoming-archive-{verify_target_build}"));
        let source = root.join("source");
        let cache = root.join("cache/gatedpkg/0.1.0");
        let store = root.join("store");
        let project = root.join("app");
        write_incoming_archive_fixture(&source, &project);
        let original = archive_directory(&source).expect("archive original package");
        fs::create_dir_all(&cache).expect("create cache");
        unpack_archive(&original, &cache).expect("stage original archive");
        let mut args = crate::cli::InstallArgs {
            path: Some(cache.clone()),
            package: None,
            manifest: PathBuf::from("cista.toml"),
            target_language: "rust".to_owned(),
            store: Some(store.clone()),
            registry: None,
            project: Some(project.clone()),
            verify_target_build: false,
        };
        super::super::install::run(&args).expect("first install establishes trust");
        fs::remove_dir_all(&cache).expect("clear original cache");
        fs::create_dir_all(&cache).expect("create pristine cache");
        unpack_archive(&original, &cache).expect("restage pristine archive");
        args.verify_target_build = verify_target_build;
        super::super::install::run(&args).expect("unchanged incoming archive remains trusted");
        let lock_before = fs::read(project.join("faber.lock")).expect("read original lock");
        let installed = store.join("gatedpkg/0.1.0");
        let installed_digest = crate::faber_lock::staged_content_sha256(&installed)
            .expect("digest original installed tree");
        let marker = root.join("build-script-ran");
        fs::write(
            source.join("rust/build.rs"),
            format!("fn main() {{ std::fs::write({marker:?}, b\"executed\").unwrap(); }}\n"),
        )
        .expect("write changed incoming build script");
        let replacement = archive_directory(&source).expect("archive changed same-version package");
        fs::remove_dir_all(&cache).expect("clear original cache");
        fs::create_dir_all(&cache).expect("create replacement cache");
        unpack_archive(&replacement, &cache).expect("stage same-name/version replacement");
        assert!(!cache.join("rust/target").exists());
        args.verify_target_build = verify_target_build;
        let result = super::super::install::run(&args);
        assert!(
            !marker.exists(),
            "changed incoming build script executed before rejection (verify_target_build={verify_target_build})"
        );
        assert!(
            !cache.join("rust/target").exists(),
            "changed incoming archive must not reach ANY Cargo command"
        );
        assert!(
            !cache.join("rust/Cargo.lock").exists(),
            "Cargo must not mutate the incoming source"
        );
        let errors = result
            .expect_err("changed incoming source must be rejected")
            .join("\n");
        assert!(
            errors.contains("mismatch"),
            "expected source mismatch, got: {errors}"
        );
        assert_eq!(
            fs::read(project.join("faber.lock")).expect("read unchanged lock"),
            lock_before
        );
        assert_eq!(
            crate::faber_lock::staged_content_sha256(&installed).expect("digest installed tree"),
            installed_digest
        );
        fs::remove_dir_all(root).expect("cleanup archive regression");
    }
}
