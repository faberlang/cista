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
