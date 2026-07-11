use std::fs;

use super::{copy_dir_clean, copy_dir_clean_with_sequence, replacement_path};

#[cfg(unix)]
#[test]
fn package_copy_rejects_symlinks() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!("cista-fs-util-{}", std::process::id()));
    let source = root.join("source");
    let destination = root.join("destination");
    fs::create_dir_all(&source).expect("create source");
    symlink("missing", source.join("link")).expect("create symlink");

    let error = copy_dir_clean(&source, &destination).expect_err("symlink should fail closed");

    assert!(error.contains("unsupported package entry"));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(unix)]
#[test]
fn package_copy_preserves_destination_when_source_is_unsupported() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!("cista-fs-util-preserve-{}", std::process::id()));
    let source = root.join("source");
    let destination = root.join("destination");
    fs::create_dir_all(&source).expect("create source");
    fs::create_dir_all(&destination).expect("create destination");
    fs::write(destination.join("installed"), "last good snapshot").expect("seed destination");
    symlink("missing", source.join("unsupported")).expect("create unsupported entry");

    let error = copy_dir_clean(&source, &destination).expect_err("reject unsupported source");

    assert!(error.contains("unsupported package entry"));
    assert_eq!(
        fs::read_to_string(destination.join("installed")).expect("read preserved destination"),
        "last good snapshot"
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn package_copy_preserves_regular_files() {
    let root = std::env::temp_dir().join(format!("cista-fs-util-files-{}", std::process::id()));
    let source = root.join("source");
    let destination = root.join("destination");
    fs::create_dir_all(source.join("nested")).expect("create source");
    fs::write(source.join("nested/file"), "contents").expect("write source file");

    copy_dir_clean(&source, &destination).expect("copy package tree");

    assert_eq!(
        fs::read_to_string(destination.join("nested/file")).expect("read copied file"),
        "contents"
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn package_copy_rejects_destination_inside_source() {
    let root = std::env::temp_dir().join(format!(
        "cista-fs-util-destination-overlap-{}",
        std::process::id()
    ));
    let source = root.join("source");
    let destination = source.join("nested/cache");
    fs::create_dir_all(&source).expect("create source");
    fs::write(source.join("payload"), "source").expect("write source payload");

    let error = copy_dir_clean(&source, &destination)
        .expect_err("destination inside source should fail closed");

    assert!(error.contains("must not overlap"), "{error}");
    assert!(!destination.exists());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn package_copy_rejects_source_inside_destination() {
    let root = std::env::temp_dir().join(format!(
        "cista-fs-util-source-overlap-{}",
        std::process::id()
    ));
    let destination = root.join("destination");
    let source = destination.join("registry/package");
    fs::create_dir_all(&source).expect("create source");
    fs::write(source.join("payload"), "source").expect("write source payload");

    let error = copy_dir_clean(&source, &destination)
        .expect_err("source inside destination should fail closed");

    assert!(error.contains("must not overlap"), "{error}");
    assert_eq!(
        fs::read_to_string(source.join("payload")).expect("read preserved source"),
        "source"
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn package_copy_rejects_preexisting_staging_directory() {
    let root = std::env::temp_dir().join(format!(
        "cista-fs-util-occupied-staging-{}",
        std::process::id()
    ));
    let source = root.join("source");
    let destination = root.join("installed");
    fs::create_dir_all(&source).expect("create source");
    fs::create_dir_all(&destination).expect("create destination");
    fs::write(source.join("new.txt"), "new").expect("write source");
    fs::write(destination.join("old.txt"), "old").expect("write destination");

    let staging = replacement_path(&destination, "incoming", 42);
    fs::create_dir_all(&staging).expect("occupy staging directory");
    fs::write(staging.join("foreign.txt"), "foreign").expect("write foreign staging file");

    let error = copy_dir_clean_with_sequence(&source, &destination, 42)
        .expect_err("occupied staging directory should fail closed");

    assert!(error.contains("failed to reserve replacement directory"));
    assert_eq!(
        fs::read_to_string(destination.join("old.txt")).expect("read preserved destination"),
        "old"
    );
    assert_eq!(
        fs::read_to_string(staging.join("foreign.txt")).expect("read foreign staging file"),
        "foreign"
    );
    fs::remove_dir_all(root).expect("remove fixture");
}
