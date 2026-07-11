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
