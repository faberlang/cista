use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use super::remove_empty_name_dir;

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
