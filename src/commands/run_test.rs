use super::*;
use crate::cli::InstallArgs;
use std::fs;

fn temp_root() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("cista-run-{}-{nanos}", std::process::id()))
}

#[test]
fn run_installed_binary_with_passthrough_argument() {
    let root = temp_root();
    let package = root.join("argcheck");
    let store = root.join("store");
    fs::create_dir_all(package.join("interfaces")).expect("create interfaces");
    fs::create_dir_all(package.join("rust/src")).expect("create rust source");
    fs::write(
        package.join("cista.toml"),
        r#"[source]
package = "argcheck"
version = "0.1.0"
faber_min = "0.38.0"
kind = "source"
role = "bin"
interfaces = "interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
source = "rust"
crate = "argcheck"

[target.compile]
emit = "binary"
crate_type = "bin"
edition = "2021"
"#,
    )
    .expect("write cista manifest");
    fs::write(
        package.join("rust/Cargo.toml"),
        r#"[package]
name = "argcheck"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("write cargo manifest");
    fs::write(
        package.join("rust/src/main.rs"),
        r#"fn main() {
    assert_eq!(std::env::args().nth(1).as_deref(), Some("proof"));
}
"#,
    )
    .expect("write binary source");

    super::super::install::run(InstallArgs {
        path: Some(package.clone()),
        package: None,
        manifest: PathBuf::from("cista.toml"),
        target_language: "rust".to_owned(),
        store: Some(store.clone()),
        registry: None,
        project: None,
        verify_target_build: false,
    })
    .expect("install binary");
    fs::remove_dir_all(&package).expect("remove source package");

    run(RunArgs {
        package: "argcheck".to_owned(),
        store: Some(store),
        args: vec!["proof".to_owned()],
    })
    .expect("run installed binary");

    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn executable_path_rejects_library_packages() {
    let manifest = manifest::CistaManifest {
        source: manifest::SourceSection {
            package: "mathesis".to_owned(),
            version: "0.1.0".to_owned(),
            faber_min: "0.38.0".to_owned(),
            kind: manifest::SourceKind::Artifact,
            role: PackageRole::Lib,
            interfaces: PathBuf::from("../../../interfaces"),
            sources: None,
        },
        target: manifest::TargetSection {
            language: "rust".to_owned(),
            mode: manifest::TargetMode::Artifact,
            binding_policy: manifest::BindingPolicy::Generated,
            source: None,
            artifact: Some(PathBuf::from("libmathesis.rlib")),
            crate_name: Some("mathesis".to_owned()),
            triple: Some("host".to_owned()),
            rustc: None,
            flags: None,
            compile: None,
        },
        bindings: Vec::new(),
    };
    let error = executable_path(&manifest, Path::new("target"), "host")
        .expect_err("library should not be runnable");
    assert!(error.contains("role `lib`"));
}
