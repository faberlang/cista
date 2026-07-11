use std::fs;

use super::copy_dir_clean;

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
