use super::*;
use clap::Parser;

#[test]
fn install_requires_exactly_one_package_source() {
    assert!(CistaCli::try_parse_from(["cista", "install", "--target-language", "rust"]).is_err());
    assert!(CistaCli::try_parse_from([
        "cista",
        "install",
        "tool@1.2.3",
        "--path",
        ".",
        "--target-language",
        "rust",
    ])
    .is_err());
}

#[test]
fn install_accepts_local_or_registry_source() {
    for arguments in [
        vec![
            "cista",
            "install",
            "--path",
            ".",
            "--target-language",
            "rust",
        ],
        vec![
            "cista",
            "install",
            "tool@1.2.3",
            "--target-language",
            "rust",
        ],
    ] {
        assert!(CistaCli::try_parse_from(arguments).is_ok());
    }
}

// --- PublishArgs ---

#[test]
fn publish_requires_path() {
    assert!(CistaCli::try_parse_from(["cista", "publish", "--manifest", "cista.toml"]).is_err());
}

#[test]
fn publish_accepts_path_only() {
    assert!(CistaCli::try_parse_from(["cista", "publish", "--path", "."]).is_ok());
}

#[test]
fn publish_requires_mutually_exclusive_registry_flags() {
    assert!(CistaCli::try_parse_from([
        "cista",
        "publish",
        "--path",
        ".",
        "--registry",
        "/tmp/registry",
        "--registry-url",
        "https://cista.dev",
    ])
    .is_err());
}

#[test]
fn publish_accepts_registry_url() {
    assert!(CistaCli::try_parse_from([
        "cista",
        "publish",
        "--path",
        ".",
        "--registry-url",
        "https://cista.dev",
    ])
    .is_ok());
}

// --- FetchArgs ---

#[test]
fn fetch_requires_package_identity() {
    assert!(CistaCli::try_parse_from(["cista", "fetch"]).is_err());
}

#[test]
fn fetch_accepts_package_identity() {
    assert!(CistaCli::try_parse_from(["cista", "fetch", "tool@1.2.3"]).is_ok());
}

#[test]
fn fetch_accepts_registry_url() {
    assert!(CistaCli::try_parse_from([
        "cista",
        "fetch",
        "tool@1.2.3",
        "--registry-url",
        "https://cista.dev",
    ])
    .is_ok());
}

#[test]
fn fetch_rejects_conflicting_registry_flags() {
    assert!(CistaCli::try_parse_from([
        "cista",
        "fetch",
        "tool@1.2.3",
        "--registry",
        "/tmp/registry",
        "--registry-url",
        "https://cista.dev",
    ])
    .is_err());
}

// --- LoginArgs ---

#[test]
fn login_accepts_default_registry_url() {
    assert!(CistaCli::try_parse_from(["cista", "login"]).is_ok());
}

#[test]
fn login_accepts_custom_registry_url() {
    assert!(CistaCli::try_parse_from([
        "cista",
        "login",
        "--registry-url",
        "https://packages.example",
    ])
    .is_ok());
}

#[test]
fn login_accepts_custom_token_env() {
    assert!(CistaCli::try_parse_from([
        "cista",
        "login",
        "--token-env",
        "MY_REGISTRY_TOKEN",
    ])
    .is_ok());
}

// --- LogoutArgs ---

#[test]
fn logout_accepts_default_registry_url() {
    assert!(CistaCli::try_parse_from(["cista", "logout"]).is_ok());
}

#[test]
fn logout_accepts_custom_registry_url() {
    assert!(CistaCli::try_parse_from([
        "cista",
        "logout",
        "--registry-url",
        "https://packages.example",
    ])
    .is_ok());
}

// --- InspectArgs ---

#[test]
fn inspect_requires_value() {
    assert!(CistaCli::try_parse_from(["cista", "inspect"]).is_err());
}

#[test]
fn inspect_accepts_path_or_package() {
    assert!(CistaCli::try_parse_from(["cista", "inspect", "."]).is_ok());
    assert!(CistaCli::try_parse_from(["cista", "inspect", "tool@1.2.3"]).is_ok());
    assert!(CistaCli::try_parse_from(["cista", "inspect", "tool"]).is_ok());
}

// --- CheckArgs ---

#[test]
fn check_accepts_default_path() {
    assert!(CistaCli::try_parse_from(["cista", "check"]).is_ok());
}

#[test]
fn check_accepts_explicit_path() {
    assert!(CistaCli::try_parse_from(["cista", "check", "/tmp/mypackage"]).is_ok());
}

#[test]
fn check_accepts_verify_target_build() {
    assert!(CistaCli::try_parse_from(["cista", "check", "--verify-target-build"]).is_ok());
}

#[test]
fn check_accepts_target_language() {
    assert!(CistaCli::try_parse_from(["cista", "check", "--target-language", "rust"]).is_ok());
}

// --- RemoveArgs ---

#[test]
fn remove_requires_package_identity() {
    assert!(CistaCli::try_parse_from(["cista", "remove"]).is_err());
}

#[test]
fn remove_accepts_package_identity() {
    assert!(CistaCli::try_parse_from(["cista", "remove", "tool@1.2.3"]).is_ok());
}

// --- RunArgs ---

#[test]
fn run_requires_package_name() {
    assert!(CistaCli::try_parse_from(["cista", "run"]).is_err());
}

#[test]
fn run_accepts_package_name() {
    assert!(CistaCli::try_parse_from(["cista", "run", "tool"]).is_ok());
}

#[test]
fn run_accepts_version_pin() {
    assert!(CistaCli::try_parse_from(["cista", "run", "tool@1.2.3"]).is_ok());
}

// --- Package subcommands ---

#[test]
fn package_list_accepts_store_arg() {
    assert!(CistaCli::try_parse_from(["cista", "package", "list"]).is_ok());
    assert!(CistaCli::try_parse_from(["cista", "package", "list", "--store", "/tmp/store"]).is_ok());
}

#[test]
fn package_show_requires_package() {
    assert!(CistaCli::try_parse_from(["cista", "package", "show"]).is_err());
    assert!(CistaCli::try_parse_from(["cista", "package", "show", "tool@1.2.3"]).is_ok());
}

// --- Init ---

#[test]
fn init_accepts_default_path() {
    assert!(CistaCli::try_parse_from(["cista", "init"]).is_ok());
}

#[test]
fn init_accepts_explicit_path() {
    assert!(CistaCli::try_parse_from(["cista", "init", "/tmp/package"]).is_ok());
}

// --- Doctor ---

#[test]
fn doctor_accepts_no_args() {
    assert!(CistaCli::try_parse_from(["cista", "doctor"]).is_ok());
}

// --- Yank ---

#[test]
fn yank_requires_package_and_version() {
    assert!(CistaCli::try_parse_from(["cista", "yank"]).is_err());
    assert!(CistaCli::try_parse_from(["cista", "yank", "tool"]).is_err());
    assert!(CistaCli::try_parse_from(["cista", "yank", "tool", "1.2.3"]).is_ok());
}
