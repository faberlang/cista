use std::fs;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{remove_empty_name_dir, run};
use crate::cli::PackageArg;
use crate::commands::shared;
use fs2::FileExt;

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
fn remove_does_not_delete_reserved_cache_namespace() {
    let root = fixture("cache-namespace");
    let store = root.join("store");
    let cached = store.join(".cache/registry/tool/1.2.3");
    fs::create_dir_all(&cached).expect("create registry cache entry");
    fs::write(cached.join("archive"), "cached package").expect("write registry cache payload");

    let error = run(PackageArg {
        package: ".cache@registry".to_owned(),
        store: Some(store.clone()),
        registry: None,
        registry_url: None,
    })
    .expect_err("reserved cache namespace must not resolve as removable package");

    assert!(error
        .iter()
        .any(|message| message.contains("is not installed")));
    assert!(
        cached.join("archive").is_file(),
        "failed removal must preserve registry cache payload"
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn remove_waits_for_store_mutation_lock() {
    let root = fixture("locked-remove");
    let store = root.join("store");
    let package = store.join("tool/1.2.3");
    fs::create_dir_all(&package).expect("create package version");
    fs::write(package.join("payload"), "installed").expect("write package payload");
    let lock = shared::acquire_store_mutation_locks(&store, None).expect("hold store lock");

    let expected_lock_path = store.join(shared::STORE_MUTATION_LOCK_FILE);
    let (attempt_tx, attempt_rx) = mpsc::channel();
    let _attempt_observer =
        shared::observe_store_lock_attempt(Arc::new(move |lock_path, lock_file| {
            if lock_path != expected_lock_path {
                return;
            }
            match lock_file.try_lock_exclusive() {
                Ok(()) => {
                    lock_file
                        .unlock()
                        .expect("release unexpected store lock acquisition");
                    panic!("remove reached an unlocked store mutation lock");
                }
                Err(_) => attempt_tx
                    .send(())
                    .expect("signal remove store lock attempt"),
            }
        }));

    let (done_tx, done_rx) = mpsc::channel();
    let remove_store = store.clone();
    let handle = std::thread::spawn(move || {
        let result = run(PackageArg {
            package: "tool@1.2.3".to_owned(),
            store: Some(remove_store),
            registry: None,
            registry_url: None,
        });
        done_tx.send(result).expect("send remove result");
    });

    attempt_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("remove thread should reach the held store mutation lock");
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
