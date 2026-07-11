use super::*;

fn temp_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "cista-credentials-{}-{}.toml",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ))
}

#[test]
fn credential_round_trip_is_scoped_by_https_origin() {
    let path = temp_path();
    store(&path, "https://cista.dev", "first").unwrap();
    store(&path, "https://packages.example", "other").unwrap();
    store(&path, "https://cista.dev", "replacement").unwrap();

    assert_eq!(
        token(&path, "https://cista.dev").unwrap().as_deref(),
        Some("replacement")
    );
    assert_eq!(
        token(&path, "https://packages.example").unwrap().as_deref(),
        Some("other")
    );
    assert!(remove(&path, "https://cista.dev").unwrap());
    assert_eq!(token(&path, "https://cista.dev").unwrap(), None);
    fs::remove_file(path).unwrap();
}

#[test]
fn credentials_reject_plain_http_and_empty_tokens() {
    let path = temp_path();
    assert!(store(&path, "http://cista.dev", "secret").is_err());
    assert!(store(&path, "https://user@cista.dev", "secret").is_err());
    assert!(store(&path, "https://cista.dev/path", "secret").is_err());
    assert!(store(&path, "https://cista.dev", " ").is_err());
    assert!(!path.exists());
}

#[cfg(unix)]
#[test]
fn credential_file_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let path = temp_path();
    store(&path, "https://cista.dev", "secret").unwrap();
    let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(token(&path, "https://cista.dev").is_err());
    fs::remove_file(path).unwrap();
}

#[test]
fn failed_replacement_removes_temporary_credentials() {
    let directory = temp_path();
    let destination = directory.join("destination");
    let temporary = directory.join("credentials.tmp");
    fs::create_dir_all(&destination).unwrap();

    let error = write_and_replace(&destination, &temporary, b"secret")
        .expect_err("replacing a directory must fail");

    assert!(error.contains("failed to replace credentials"));
    assert!(!temporary.exists());
    assert!(destination.is_dir());
    fs::remove_dir_all(directory).unwrap();
}
