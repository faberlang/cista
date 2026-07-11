use std::path::Path;

use crate::manifest::Binding;

use super::{interface_files, is_interface_file, runtime_binding_lines};

#[test]
fn interface_files_are_identified_by_path_components() {
    assert!(is_interface_file(Path::new("interfaces/mathesis.fab")));
    assert!(is_interface_file(Path::new("interfaces/solum/path.fab")));
    assert!(!is_interface_file(Path::new("interfaces/readme.md")));
    assert!(!is_interface_file(Path::new(
        "other/interfaces/mathesis.fab"
    )));
    assert!(!is_interface_file(Path::new("interfaces.fab")));
}

#[test]
fn package_interfaces_exclude_non_interface_files() {
    let files = vec![
        "cista.toml".into(),
        "interfaces/solum.fab".into(),
        "targets/rust/host/cista.toml".into(),
    ];

    assert_eq!(interface_files(files), [Path::new("interfaces/solum.fab")]);
    assert!(interface_files(Vec::new()).is_empty());
}

#[test]
fn runtime_bindings_are_formatted_for_inspection() {
    let bindings = [Binding {
        source_module: "solum".to_owned(),
        source_symbol: "via".to_owned(),
        target: "norma::solum::via".to_owned(),
    }];

    assert_eq!(
        runtime_binding_lines(&bindings),
        ["solum#via -> norma::solum::via"]
    );
    assert!(runtime_binding_lines(&[]).is_empty());
}
