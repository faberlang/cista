use super::*;
use crate::cli::InstallArgs;
use crate::faber_lock::read_lock;
use std::process::Command;

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
        path: Some(package.clone()),
        package: None,
        manifest: PathBuf::from("cista.toml"),
        target_language: "rust".to_owned(),
        store: Some(store.clone()),
        registry: None,
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

#[test]
fn install_binary_materializes_runnable_host_entry() {
    let root = temp_root("binary");
    let package = root.join("true");
    let store = root.join("store");
    fs::create_dir_all(package.join("interfaces")).expect("create interfaces");
    fs::create_dir_all(package.join("rust/src")).expect("create rust source");
    fs::write(
        package.join("cista.toml"),
        r#"[source]
package = "true"
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
crate = "true"

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
name = "true"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("write cargo manifest");
    fs::write(package.join("rust/src/main.rs"), "fn main() {}\n").expect("write binary source");

    run(InstallArgs {
        path: Some(package),
        package: None,
        manifest: PathBuf::from("cista.toml"),
        target_language: "rust".to_owned(),
        store: Some(store.clone()),
        registry: None,
        project: None,
        verify_target_build: false,
    })
    .expect("install binary");

    let triple = rust_target::rust_host_triple().expect("host triple");
    let target = store.join("true/0.1.0/targets/rust").join(triple);
    let executable = target.join("true");
    assert!(executable.is_file(), "installed executable should exist");
    assert!(Command::new(&executable)
        .status()
        .expect("run installed binary")
        .success());
    let installed = crate::manifest::read_manifest(&target.join("cista.toml"))
        .expect("read installed manifest");
    assert_eq!(installed.source.role, crate::manifest::PackageRole::Bin);
    assert_eq!(
        installed.target.artifact.as_deref(),
        Some(Path::new("true"))
    );

    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn install_meta_expands_exact_local_dependencies() {
    let root = temp_root("meta");
    let packages = root.join("packages");
    let binary = packages.join("true");
    let meta = packages.join("coreutils");
    let store = root.join("store");
    fs::create_dir_all(binary.join("interfaces")).expect("create interfaces");
    fs::create_dir_all(binary.join("rust/src")).expect("create rust source");
    fs::create_dir_all(&meta).expect("create meta package");
    fs::write(
        binary.join("cista.toml"),
        r#"[source]
package = "true"
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
crate = "true"

[target.compile]
emit = "binary"
crate_type = "bin"
edition = "2021"
"#,
    )
    .expect("write binary manifest");
    fs::write(
        binary.join("rust/Cargo.toml"),
        r#"[package]
name = "true"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("write cargo manifest");
    fs::write(binary.join("rust/src/main.rs"), "fn main() {}\n").expect("write binary source");
    fs::write(
        meta.join("cista.toml"),
        r#"[source]
package = "coreutils"
version = "0.1.0"
role = "meta"

[[dependencies]]
package = "true"
version = "0.1.0"
path = "../true"
"#,
    )
    .expect("write meta manifest");

    run(InstallArgs {
        path: Some(meta),
        package: None,
        manifest: PathBuf::from("cista.toml"),
        target_language: "rust".to_owned(),
        store: Some(store.clone()),
        registry: None,
        project: None,
        verify_target_build: false,
    })
    .expect("install meta package");

    assert!(store.join("true/0.1.0").is_dir());
    let installed_meta = store.join("coreutils/0.1.0/cista.toml");
    assert!(installed_meta.is_file());
    let parsed = crate::manifest::read_meta_manifest(&installed_meta)
        .expect("read installed meta")
        .expect("manifest should be meta");
    assert_eq!(parsed.dependencies.len(), 1);
    assert_eq!(parsed.dependencies[0].package, "true");
    assert!(
        parsed.dependencies[0].path.is_none(),
        "installed meta pins must not retain source-relative paths"
    );

    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn ordinary_manifest_rejects_meta_role() {
    let root = temp_root("ordinary-meta-role");
    let manifest_path = root.join("cista.toml");
    fs::write(
        &manifest_path,
        r#"[source]
package = "not-meta"
version = "0.1.0"
faber_min = "0.38.0"
kind = "source"
role = "meta"
interfaces = "interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
"#,
    )
    .expect("write invalid ordinary manifest");

    let error = crate::manifest::read_manifest(&manifest_path)
        .expect_err("ordinary schema must reject the meta role");
    assert!(error.contains("unknown variant `meta`"), "{error}");

    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn install_real_norma_platform_default_builds_nested_import_without_dependency() {
    let root = temp_root("real-norma-platform-default");
    let store = root.join("store");
    let project = root.join("app");
    let fake_library_home = root.join("fake-library-home");
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("faberlang workspace")
        .to_path_buf();
    let norma = workspace.join("norma");
    let faber_manifest = workspace.join("faber/Cargo.toml");

    fs::create_dir_all(project.join("src")).expect("create project src");
    fs::create_dir_all(fake_library_home.join("norma/src/solum"))
        .expect("create fake library home");
    fs::write(
        fake_library_home.join("norma/src/solum/path.fab"),
        "functio nomen(textus via) → textus { redde \"wrong\" }\n",
    )
    .expect("write fake fallback interface");
    fs::write(
        project.join(PROJECT_MANIFEST),
        r#"[package]
name = "norma-lock-proof"
version = "0.1.0"
edition = "2026"

[paths]
source = "src"
entry = "main.fab"
"#,
    )
    .expect("write project manifest");
    fs::write(
        project.join("src/main.fab"),
        r#"importa ex "norma:solum/path" privata path

incipit {
    nota path.nomen("/tmp/file.txt")
}
"#,
    )
    .expect("write project source");

    run(InstallArgs {
        path: Some(norma.clone()),
        package: None,
        manifest: PathBuf::from("cista.toml"),
        target_language: "rust".to_owned(),
        store: Some(store.clone()),
        registry: None,
        project: Some(project.clone()),
        verify_target_build: false,
    })
    .expect("install real norma");

    let lock = read_lock(&project.join(faber_lock::LOCK_FILE)).expect("read lock");
    let norma_lock = lock
        .packages
        .iter()
        .find(|package| package.name == "norma")
        .expect("norma lock record");
    assert_eq!(norma_lock.version, "0.1.0");
    assert!(
        PathBuf::from(&norma_lock.interface_root)
            .join("solum/path.fab")
            .is_file(),
        "real norma nested interface should be installed"
    );

    for command in ["check", "build"] {
        let output = Command::new("cargo")
            .args([
                "run",
                "--manifest-path",
                faber_manifest.to_str().expect("faber manifest path"),
                "--",
                command,
                project.to_str().expect("project path"),
            ])
            .env("FABER_LIBRARY_HOME", &fake_library_home)
            .output()
            .unwrap_or_else(|err| panic!("spawn faber {command}: {err}"));
        assert!(
            output.status.success(),
            "faber {command} should consume locked real Norma before fallback\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fs::remove_dir_all(root).expect("cleanup temp root");
}
