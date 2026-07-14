use std::fs;
use std::sync::mpsc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{remove_empty_name_dir, run};
use crate::cli::PackageArg;
use crate::commands::shared;

fn fixture(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cista-remove-{name}-{}-{nanos}",
        std::process::id()
    ))
}

#[test]
fn empty_package_name_directory_is_removed() {
    let name_dir = fixture("empty");
    fs::create_dir_all(&name_dir).expect("create package directory");

    remove_empty_name_dir(&name_dir).expect("remove empty package directory");

    assert!(!name_dir.exists());
}

#[test]
fn package_name_directory_with_another_version_is_preserved() {
    let name_dir = fixture("nonempty");
    fs::create_dir_all(name_dir.join("2.0.0")).expect("create remaining version");

    remove_empty_name_dir(&name_dir).expect("preserve nonempty package directory");

    assert!(name_dir.join("2.0.0").is_dir());
    fs::remove_dir_all(name_dir).expect("remove fixture");
}

#[test]
fn remove_waits_for_store_mutation_lock() {
    let root = fixture("locked-remove");
    let store = root.join("store");
    let package = store.join("tool/1.2.3");
    fs::create_dir_all(&package).expect("create package version");
    fs::write(package.join("payload"), "installed").expect("write package payload");
    let lock = shared::acquire_store_mutation_locks(&store, None).expect("hold store lock");

    let (ready_tx, ready_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let remove_store = store.clone();
    let handle = std::thread::spawn(move || {
        ready_tx.send(()).expect("signal remove thread ready");
        let result = run(PackageArg {
            package: "tool@1.2.3".to_owned(),
            store: Some(remove_store),
            registry: None,
            registry_url: None,
        });
        done_tx.send(result).expect("send remove result");
    });

    ready_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("remove thread should start");
    std::thread::sleep(Duration::from_millis(50));
    assert!(
        package.is_dir(),
        "remove must not mutate the package while the store lock is held"
    );
    assert!(
        done_rx.try_recv().is_err(),
        "remove should still be waiting for the held store lock"
    );

    drop(lock);
    done_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("remove should complete after lock release")
        .expect("remove should succeed after lock release");
    handle.join().expect("remove thread should not panic");
    assert!(!package.exists());

    fs::remove_dir_all(root).expect("remove fixture");
}
