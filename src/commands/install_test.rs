use super::*;
use crate::cli::InstallArgs;
use crate::faber_lock::read_lock;

fn temp_root(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("cista-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path).expect("create temp root");
    path
}

#[test]
fn install_norma_platform_default_snapshots_src_interfaces_without_artifact() {
    let root = temp_root("norma-platform-default");
    let package = root.join("norma");
    let store = root.join("store");
    let project = root.join("app");
    fs::create_dir_all(package.join("src/solum")).expect("create package src");
    fs::create_dir_all(&project).expect("create project");
    fs::write(
        package.join("cista.toml"),
        r#"[source]
package = "norma"
version = "0.1.0"
faber_min = "0.38.0"
kind = "source"
interfaces = "src"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
crate = "norma"
"#,
    )
    .expect("write cista manifest");
    fs::write(
        package.join("src/solum.fab"),
        "functio lege(textus via) → textus { redde via }\n",
    )
    .expect("write root interface");
    fs::write(
        package.join("src/solum/path.fab"),
        "functio nomen(textus via) → textus { redde via }\n",
    )
    .expect("write nested interface");
    fs::write(
        project.join(PROJECT_MANIFEST),
        r#"[package]
name = "app"
version = "0.1.0"
edition = "2026"

[paths]
source = "src"
entry = "main.fab"
"#,
    )
    .expect("write project manifest");

    run(InstallArgs {
        path: package.clone(),
        manifest: PathBuf::from("cista.toml"),
        target_language: "rust".to_owned(),
        store: Some(store.clone()),
        project: Some(project.clone()),
        verify_target_build: false,
    })
    .expect("install norma");

    let installed_root = store.join("norma/0.1.0");
    assert!(installed_root.join("interfaces/solum.fab").is_file());
    assert!(installed_root.join("interfaces/solum/path.fab").is_file());

    let lock = read_lock(&project.join(faber_lock::LOCK_FILE)).expect("read lock");
    let norma = lock
        .packages
        .iter()
        .find(|package| package.name == "norma")
        .expect("norma lock record");
    assert_eq!(norma.version, "0.1.0");
    assert_eq!(norma.kind, "source");
    assert!(
        norma.artifact.is_empty(),
        "interfaces-only norma must not invent an artifact"
    );
    assert_eq!(
        PathBuf::from(&norma.interface_root),
        installed_root
            .join("interfaces")
            .canonicalize()
            .expect("canonical interfaces")
    );

    let installed_manifest = crate::manifest::read_manifest(&PathBuf::from(&norma.target_manifest))
        .expect("read installed target manifest");
    assert!(installed_manifest.target.artifact.is_none());
    assert!(installed_manifest.bindings.is_empty());
    assert_eq!(
        installed_manifest.target.binding_policy,
        BindingPolicy::Generated
    );

    fs::remove_dir_all(root).expect("cleanup temp root");
}
