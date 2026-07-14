use std::path::Path;

use crate::cli::{PackageArg, PackageCommand, PackageSubcommand};
use crate::manifest::Binding;

use super::{interface_files, is_interface_file, runtime_binding_lines};

fn temp_root(name: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cista-package-{name}-{}-{nanos}",
        std::process::id()
    ))
}

fn package_arg(store: &Path) -> PackageArg {
    PackageArg {
        package: ".cache@registry".to_owned(),
        store: Some(store.to_path_buf()),
        registry: None,
        registry_url: None,
    }
}

#[test]
fn package_inspection_does_not_resolve_reserved_cache_namespace() {
    let root = temp_root("cache-namespace");
    let store = root.join("store");
    let cached = store.join(".cache/registry/tool/1.2.3");
    std::fs::create_dir_all(&cached).expect("create registry cache entry");
    std::fs::write(cached.join("archive"), "cached package").expect("write registry cache payload");

    for command in [
        PackageSubcommand::Show(package_arg(&store)),
        PackageSubcommand::Files(package_arg(&store)),
        PackageSubcommand::Interfaces(package_arg(&store)),
        PackageSubcommand::Runtimes(package_arg(&store)),
    ] {
        let error = super::run(PackageCommand { command })
            .expect_err("reserved cache namespace must not resolve for package inspection");
        assert!(error
            .iter()
            .any(|message| message.contains("is not installed")));
        assert!(
            cached.join("archive").is_file(),
            "failed inspection must preserve registry cache payload"
        );
    }

    std::fs::remove_dir_all(root).expect("cleanup temp root");
}

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
