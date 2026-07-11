use std::fs;

use super::{list_package_files, read_any_target_manifest, utf8_directory_name, InstalledPackage};

fn temporary_dir(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "cista-store-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should follow the Unix epoch")
            .as_nanos()
    ));
    fs::create_dir_all(&path).expect("temporary directory should be created");
    path
}

fn installed_package(root: &std::path::Path) -> InstalledPackage {
    InstalledPackage {
        name: "demo".to_owned(),
        version: "1.0.0".to_owned(),
        package_root: root.to_path_buf(),
        interfaces_dir: root.join("interfaces"),
        targets_dir: root.join("targets"),
    }
}

#[cfg(unix)]
#[test]
fn package_file_listing_rejects_symlinks() {
    use std::os::unix::fs::symlink;

    let root = temporary_dir("symlink");
    fs::write(root.join("regular.txt"), "inside").expect("fixture should be written");
    symlink("regular.txt", root.join("alias.txt")).expect("symlink should be created");

    let error = list_package_files(&root).expect_err("symlink should fail closed");

    assert!(error.contains("unsupported symlink"), "{error}");
    fs::remove_dir_all(root).expect("temporary directory should be removed");
}

#[cfg(unix)]
#[test]
fn package_file_listing_rejects_special_entries() {
    use std::os::unix::net::UnixListener;

    let root = std::path::PathBuf::from(format!("/tmp/cista-socket-{}", std::process::id()));
    fs::create_dir_all(&root).expect("temporary directory should be created");
    let socket_path = root.join("package.sock");
    let _listener = UnixListener::bind(&socket_path).expect("fixture socket should be created");

    let error = list_package_files(&root).expect_err("special entry should fail closed");

    assert!(error.contains("unsupported entry"), "{error}");
    fs::remove_dir_all(root).expect("temporary directory should be removed");
}

#[test]
fn malformed_target_manifest_is_reported() {
    let root = temporary_dir("manifest");
    let target = root.join("targets/rust/test-triple");
    fs::create_dir_all(&target).expect("target directory should be created");
    fs::write(target.join("cista.toml"), "not = [valid").expect("fixture should be written");

    let error = read_any_target_manifest(&installed_package(&root))
        .expect_err("malformed manifest should be reported");

    assert!(error.contains("failed to parse"), "{error}");
    fs::remove_dir_all(root).expect("temporary directory should be removed");
}

#[cfg(unix)]
#[test]
fn package_directory_name_rejects_non_utf8_input() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let invalid_name = OsString::from_vec(vec![b'p', 0xff]);

    let error = utf8_directory_name(&std::path::PathBuf::from(invalid_name), "package")
        .expect_err("non-UTF-8 package name should fail closed");

    assert!(
        error.contains("package directory name is not UTF-8"),
        "{error}"
    );
}
