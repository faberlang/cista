use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_directory() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("cista-faber-lock-{}-{nonce}", std::process::id()))
}

fn package(name: &str, version: &str) -> LockedPackage {
    LockedPackage {
        name: name.to_owned(),
        version: version.to_owned(),
        source: format!("path:/sources/{name}"),
        package_root: format!("/store/{name}/{version}"),
        kind: "lib".to_owned(),
        target_language: "rust".to_owned(),
        target_triple: "test-target".to_owned(),
        target_manifest: format!("/store/{name}/{version}/targets/rust/test-target/cista.toml"),
        interface_root: format!("/store/{name}/{version}/interfaces"),
        artifact: String::new(),
        crate_name: name.to_owned(),
        rustc: "rustc test".to_owned(),
    }
}

fn replacement_lock() -> FaberLock {
    FaberLock {
        packages: vec![package("new", "2.0.0")],
    }
}

fn seed_existing_lock(path: &Path) -> LockedPackage {
    let existing = package("old", "1.0.0");
    write_lock(
        path,
        &FaberLock {
            packages: vec![existing.clone()],
        },
    )
    .expect("seed existing lock");
    existing
}

fn temporary_lock_files(directory: &Path) -> Vec<PathBuf> {
    fs::read_dir(directory)
        .expect("list lock directory")
        .filter_map(|entry| {
            let path = entry.expect("read lock directory entry").path();
            let name = path.file_name()?.to_str()?;
            (name.starts_with("faber.lock.") && name.ends_with(".tmp")).then_some(path)
        })
        .collect()
}

fn assert_existing_lock_is_preserved(path: &Path, existing: &LockedPackage) {
    assert_eq!(
        read_lock(path).expect("read preserved lock").packages,
        vec![existing.clone()]
    );
}

#[test]
fn write_lock_replaces_existing_file_with_stable_ordering() {
    let directory = temporary_directory();
    let path = directory.join(LOCK_FILE);
    fs::create_dir_all(&directory).expect("create test directory");
    fs::write(&path, "invalid previous contents").expect("seed previous lock");

    let lock = FaberLock {
        packages: vec![package("zeta", "1.0.0"), package("alpha", "2.0.0")],
    };
    write_lock(&path, &lock).expect("replace lock");

    let written = fs::read_to_string(&path).expect("read replaced lock");
    assert!(
        written.find("alpha").expect("alpha entry") < written.find("zeta").expect("zeta entry")
    );
    assert_eq!(
        read_lock(&path)
            .expect("parse replaced lock")
            .packages
            .len(),
        2
    );
    assert_eq!(
        fs::read_dir(&directory)
            .expect("list test directory")
            .count(),
        1
    );

    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn write_lock_create_failure_preserves_existing_lock_without_temp_file() {
    let directory = temporary_directory();
    let path = directory.join(LOCK_FILE);
    fs::create_dir_all(&directory).expect("create test directory");
    let existing = seed_existing_lock(&path);

    inject_write_and_replace_fault(WriteAndReplaceFault::BeforeCreate);
    let error = write_lock(&path, &replacement_lock())
        .expect_err("injected temporary creation failure should fail");

    assert!(
        error.contains("injected failure before creating"),
        "{error}"
    );
    assert_existing_lock_is_preserved(&path, &existing);
    assert!(
        temporary_lock_files(&directory).is_empty(),
        "create failure must not leave a temporary lock file"
    );

    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn write_lock_write_failure_preserves_existing_lock_and_removes_temp_file() {
    let directory = temporary_directory();
    let path = directory.join(LOCK_FILE);
    fs::create_dir_all(&directory).expect("create test directory");
    let existing = seed_existing_lock(&path);

    inject_write_and_replace_fault(WriteAndReplaceFault::Write);
    let error = write_lock(&path, &replacement_lock())
        .expect_err("injected temporary write failure should fail");

    assert!(error.contains("injected failure while writing"), "{error}");
    assert_existing_lock_is_preserved(&path, &existing);
    assert!(
        temporary_lock_files(&directory).is_empty(),
        "write failure must remove the temporary lock file"
    );

    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn write_lock_rename_failure_preserves_existing_lock_and_removes_temp_file() {
    let directory = temporary_directory();
    let path = directory.join(LOCK_FILE);
    fs::create_dir_all(&directory).expect("create test directory");
    let existing = seed_existing_lock(&path);

    inject_write_and_replace_fault(WriteAndReplaceFault::Rename);
    let error = write_lock(&path, &replacement_lock())
        .expect_err("injected temporary rename failure should fail");

    assert!(
        error.contains("injected failure while replacing"),
        "{error}"
    );
    assert_existing_lock_is_preserved(&path, &existing);
    assert!(
        temporary_lock_files(&directory).is_empty(),
        "rename failure must remove the temporary lock file"
    );

    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn write_lock_cleanup_failure_preserves_existing_lock_and_surfaces_cleanup_error() {
    let directory = temporary_directory();
    let path = directory.join(LOCK_FILE);
    fs::create_dir_all(&directory).expect("create test directory");
    let existing = seed_existing_lock(&path);

    inject_write_and_replace_faults(vec![
        WriteAndReplaceFault::Rename,
        WriteAndReplaceFault::Cleanup,
    ]);
    let error = write_lock(&path, &replacement_lock())
        .expect_err("injected temporary cleanup failure should fail");

    assert!(
        error.contains("injected failure while replacing")
            && error.contains("failed to remove")
            && error.contains("injected cleanup failure"),
        "{error}"
    );
    assert_existing_lock_is_preserved(&path, &existing);
    assert_eq!(
        temporary_lock_files(&directory).len(),
        1,
        "cleanup failure should leave the temporary lock file for diagnostics"
    );

    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn write_lock_parent_sync_failure_reports_committed_lock_risk() {
    let directory = temporary_directory();
    let path = directory.join(LOCK_FILE);
    fs::create_dir_all(&directory).expect("create test directory");
    seed_existing_lock(&path);

    inject_write_and_replace_fault(WriteAndReplaceFault::SyncParent);
    let error = write_lock(&path, &replacement_lock())
        .expect_err("injected parent directory sync failure should fail");

    assert!(
        error.contains("injected failure while syncing parent directory after replacing"),
        "{error}"
    );
    assert_eq!(
        read_lock(&path).expect("read replaced lock after sync failure"),
        replacement_lock(),
        "rename has committed before parent directory sync failure is reported"
    );
    assert!(
        temporary_lock_files(&directory).is_empty(),
        "committed replacement must not leave a temporary lock file"
    );

    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn upsert_package_replaces_only_the_matching_name() {
    let mut lock = FaberLock {
        packages: vec![package("alpha", "1.0.0"), package("beta", "1.0.0")],
    };

    upsert_package(&mut lock, package("alpha", "2.0.0"));

    assert_eq!(
        lock.packages,
        vec![package("alpha", "2.0.0"), package("beta", "1.0.0")]
    );
}

#[test]
fn upsert_package_removes_duplicate_matching_names() {
    let mut duplicate = package("tool", "0.9.0");
    duplicate.package_root = "/malformed/stale/tool".to_owned();
    duplicate.target_manifest = "/malformed/stale/tool/cista.toml".to_owned();
    let mut lock = FaberLock {
        packages: vec![
            package("alpha", "1.0.0"),
            package("tool", "1.0.0"),
            package("beta", "1.0.0"),
            duplicate,
            package("tool", "1.1.0"),
        ],
    };

    upsert_package(&mut lock, package("tool", "2.0.0"));

    assert_eq!(
        lock.packages,
        vec![
            package("alpha", "1.0.0"),
            package("tool", "2.0.0"),
            package("beta", "1.0.0")
        ]
    );
}
