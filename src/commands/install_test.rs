use super::*;
use crate::cli::InstallArgs;
use crate::faber_lock::read_lock;
use fs2::FileExt;
use std::fs::OpenOptions;
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

fn faberlang_workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|root| {
            root.join("faber/Cargo.toml").is_file() && root.join("norma/cista.toml").is_file()
        })
        .expect("faberlang workspace containing faber and norma")
        .to_path_buf()
}

#[test]
fn install_lock_excludes_other_handles() {
    let root = temp_root("install-lock");
    let store = root.join("store");
    let lock = shared::acquire_store_mutation_locks(&store, None).expect("acquire install lock");
    let lock_path = store.join(shared::STORE_MUTATION_LOCK_FILE);
    let contender = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .expect("open competing lock handle");

    assert!(
        contender.try_lock_exclusive().is_err(),
        "a second handle must not enter the install critical section"
    );
    drop(lock);
    contender
        .try_lock_exclusive()
        .expect("lock should be released when the guard drops");
    contender.unlock().expect("unlock competing handle");

    fs::remove_dir_all(root).expect("cleanup temp root");
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
    assert!(installed_manifest.target.triple.is_none());
    assert!(installed_manifest.target.rustc.is_none());
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
fn install_meta_rejects_dependency_paths_outside_package_collection() {
    let root = temp_root("meta-contained-path");
    let packages = root.join("packages");
    let meta = packages.join("meta");
    let outside = root.join("outside");
    let store = root.join("store");
    fs::create_dir_all(&meta).expect("create meta package");
    fs::create_dir_all(&outside).expect("create outside package");
    fs::write(
        meta.join("cista.toml"),
        r#"[source]
package = "meta"
version = "0.1.0"
role = "meta"

[[dependencies]]
package = "outside"
version = "0.1.0"
path = "../../outside"
"#,
    )
    .expect("write meta manifest");

    let error = run(InstallArgs {
        path: Some(meta),
        package: None,
        manifest: PathBuf::from("cista.toml"),
        target_language: "rust".to_owned(),
        store: Some(store.clone()),
        registry: None,
        project: None,
        verify_target_build: false,
    })
    .expect_err("meta dependency outside package collection must fail");

    assert!(error.iter().any(|message| {
        message.contains(
            "meta dependency `outside@0.1.0` path resolves outside package collection root",
        )
    }));
    assert!(
        !store.join("outside/0.1.0").exists(),
        "rejected dependency must not install a package snapshot"
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

fn write_interfaces_only_package(package: &Path, name: &str) {
    fs::create_dir_all(package.join("interfaces")).expect("create package interfaces");
    fs::write(
        package.join("cista.toml"),
        format!(
            r#"[source]
package = "{name}"
version = "0.1.0"
faber_min = "0.38.0"
kind = "source"
interfaces = "interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
"#
        ),
    )
    .expect("write interfaces-only manifest");
    fs::write(
        package.join("interfaces/example.fab"),
        "functio lege() → nihil { redde nihil }\n",
    )
    .expect("write package interface");
}

#[test]
fn install_commit_failure_preserves_existing_snapshot() {
    let root = temp_root("install-commit-failure");
    let package = root.join("package");
    let store = root.join("store");
    write_interfaces_only_package(&package, "example");

    let installed = store.join("example/0.1.0");
    fs::create_dir_all(installed.join("interfaces")).expect("create old interfaces");
    fs::create_dir_all(installed.join("targets/rust/old")).expect("create old target");
    fs::write(installed.join("interfaces/old.fab"), "old interface\n")
        .expect("write old interface");
    fs::write(installed.join("targets/rust/old/marker"), "old target\n").expect("write old target");

    fs_util::inject_commit_failure(&installed.canonicalize().expect("canonical installed path"));
    let error = run(InstallArgs {
        path: Some(package),
        package: None,
        manifest: PathBuf::from("cista.toml"),
        target_language: "rust".to_owned(),
        store: Some(store.clone()),
        registry: None,
        project: None,
        verify_target_build: false,
    })
    .expect_err("injected package commit failure should fail install");
    assert!(error
        .iter()
        .any(|message| message.contains("injected failure")));
    assert_eq!(
        fs::read_to_string(installed.join("interfaces/old.fab")).expect("read old interface"),
        "old interface\n"
    );
    assert_eq!(
        fs::read_to_string(installed.join("targets/rust/old/marker")).expect("read old target"),
        "old target\n"
    );
    assert!(!installed.join("interfaces/example.fab").exists());
    assert_eq!(
        fs::read_dir(store.join("example"))
            .expect("read package versions")
            .count(),
        1,
        "failed install must not leave a sibling staging directory"
    );

    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn install_finalize_failure_after_backup_disposal_preserves_committed_snapshot() {
    let root = temp_root("install-finalize-post-commit");
    let package = root.join("package");
    let store = root.join("store");
    write_interfaces_only_package(&package, "example");

    let installed = store.join("example/0.1.0");
    fs::create_dir_all(installed.join("interfaces")).expect("create old interfaces");
    fs::write(installed.join("interfaces/old.fab"), "old interface\n")
        .expect("write old interface");

    fs_util::inject_finalize_sync_failure(
        &installed
            .canonicalize()
            .expect("canonical installed package path"),
    );
    let error = run(InstallArgs {
        path: Some(package),
        package: None,
        manifest: PathBuf::from("cista.toml"),
        target_language: "rust".to_owned(),
        store: Some(store.clone()),
        registry: None,
        project: None,
        verify_target_build: false,
    })
    .expect_err("post-commit finalize failure should still report error");
    assert!(error
        .iter()
        .any(|message| message.contains("injected failure after committing replacement")));

    assert_eq!(
        fs::read_to_string(installed.join("interfaces/example.fab"))
            .expect("read committed interface"),
        "functio lege() \u{2192} nihil { redde nihil }\n"
    );
    assert!(
        !installed.join("interfaces/old.fab").exists(),
        "post-commit finalize failure must not roll back a committed package snapshot"
    );

    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn install_lock_failure_rolls_back_new_store_snapshot() {
    let root = temp_root("install-lock-failure");
    let package = root.join("package");
    let project = root.join("project");
    let store = root.join("store");
    write_interfaces_only_package(&package, "example");
    let installed = store.join("example/0.1.0");
    fs::create_dir_all(installed.join("interfaces")).expect("create existing snapshot");
    fs::write(installed.join("interfaces/old.fab"), "old interface\n")
        .expect("write existing interface");
    fs::create_dir_all(&project).expect("create project");
    fs::write(
        project.join(PROJECT_MANIFEST),
        r#"[package]
name = "app"
version = "0.1.0"
edition = "2026"

[dependencies]
example = "0.1.0"
"#,
    )
    .expect("write project manifest");
    fs::create_dir(project.join(faber_lock::LOCK_FILE)).expect("occupy lock path");

    let error = run(InstallArgs {
        path: Some(package),
        package: None,
        manifest: PathBuf::from("cista.toml"),
        target_language: "rust".to_owned(),
        store: Some(store.clone()),
        registry: None,
        project: Some(project),
        verify_target_build: false,
    })
    .expect_err("lock rewrite failure should fail install");

    assert!(error
        .iter()
        .any(|message| message.contains("failed to read") && message.contains("faber.lock")));
    assert_eq!(
        fs::read_to_string(installed.join("interfaces/old.fab")).expect("read existing interface"),
        "old interface\n"
    );
    assert!(!installed.join("interfaces/example.fab").exists());

    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn meta_commit_failure_preserves_existing_snapshot() {
    let root = temp_root("meta-commit-failure");
    let packages = root.join("packages");
    let dependency = packages.join("dependency");
    let meta = packages.join("meta");
    let store = root.join("store");
    write_interfaces_only_package(&dependency, "dependency");
    fs::create_dir_all(&meta).expect("create meta package");
    fs::write(
        meta.join("cista.toml"),
        r#"[source]
package = "meta"
version = "0.1.0"
role = "meta"

[[dependencies]]
package = "dependency"
version = "0.1.0"
path = "../dependency"
"#,
    )
    .expect("write meta manifest");

    let installed = store.join("meta/0.1.0");
    fs::create_dir_all(&installed).expect("create old meta snapshot");
    fs::write(installed.join("cista.toml"), "old meta snapshot\n")
        .expect("write old meta snapshot");

    fs_util::inject_commit_failure(&installed.canonicalize().expect("canonical installed path"));
    let error = run(InstallArgs {
        path: Some(meta),
        package: None,
        manifest: PathBuf::from("cista.toml"),
        target_language: "rust".to_owned(),
        store: Some(store.clone()),
        registry: None,
        project: None,
        verify_target_build: false,
    })
    .expect_err("injected meta commit failure should fail install");
    assert!(error
        .iter()
        .any(|message| message.contains("injected failure")));
    assert_eq!(
        fs::read_to_string(installed.join("cista.toml")).expect("read old meta snapshot"),
        "old meta snapshot\n"
    );
    assert!(
        !store.join("dependency/0.1.0").exists(),
        "failed meta install must roll back dependency snapshots"
    );
    assert_eq!(
        fs::read_dir(store.join("meta"))
            .expect("read meta versions")
            .count(),
        1,
        "failed meta install must not leave a sibling staging directory"
    );

    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn meta_finalize_failure_after_backup_disposal_preserves_committed_snapshot() {
    let root = temp_root("meta-finalize-post-commit");
    let packages = root.join("packages");
    let dependency = packages.join("dependency");
    let meta = packages.join("meta");
    let store = root.join("store");
    write_interfaces_only_package(&dependency, "dependency");
    fs::create_dir_all(&meta).expect("create meta package");
    fs::write(
        meta.join("cista.toml"),
        r#"[source]
package = "meta"
version = "0.1.0"
role = "meta"

[[dependencies]]
package = "dependency"
version = "0.1.0"
path = "../dependency"
"#,
    )
    .expect("write meta manifest");

    let installed = store.join("meta/0.1.0");
    fs::create_dir_all(&installed).expect("create old meta snapshot");
    fs::write(installed.join("cista.toml"), "old meta snapshot\n")
        .expect("write old meta snapshot");

    fs_util::inject_finalize_sync_failure(
        &installed
            .canonicalize()
            .expect("canonical installed meta path"),
    );
    let error = run(InstallArgs {
        path: Some(meta),
        package: None,
        manifest: PathBuf::from("cista.toml"),
        target_language: "rust".to_owned(),
        store: Some(store.clone()),
        registry: None,
        project: None,
        verify_target_build: false,
    })
    .expect_err("post-commit finalize failure should still report error");
    assert!(error
        .iter()
        .any(|message| message.contains("injected failure after committing replacement")));

    let parsed = crate::manifest::read_meta_manifest(&installed.join("cista.toml"))
        .expect("read committed meta manifest")
        .expect("committed manifest should be meta");
    assert_eq!(parsed.source.package, "meta");
    assert_eq!(parsed.dependencies.len(), 1);
    assert!(
        store.join("dependency/0.1.0").is_dir(),
        "committed dependency must not be rolled back after meta commit is durable enough to keep"
    );

    fs::remove_dir_all(root).expect("cleanup temp root");
}

#[test]
fn install_real_norma_platform_default_builds_nested_import_without_dependency() {
    let root = temp_root("real-norma-platform-default");
    let store = root.join("store");
    let project = root.join("app");
    let fake_library_home = root.join("fake-library-home");
    let workspace = faberlang_workspace();
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
