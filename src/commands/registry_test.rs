use super::*;
use std::fs;

fn temp_root() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("cista-registry-{}-{nanos}", std::process::id()))
}

#[test]
fn publish_and_fetch_exact_package_snapshot() {
    let root = temp_root();
    let source = root.join("source");
    let registry = root.join("registry");
    let store = root.join("store");
    fs::create_dir_all(source.join("interfaces")).expect("create interfaces");
    fs::create_dir_all(source.join("rust/src")).expect("create rust source");
    fs::write(
        source.join("interfaces/tool.fab"),
        "functio main() → nihil\n",
    )
    .expect("write interface");
    fs::write(
        source.join("cista.toml"),
        r#"[source]
package = "tool"
version = "1.2.3"
faber_min = "0.38.0"
kind = "source"
role = "bin"
interfaces = "interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
source = "rust"
crate = "tool"

[target.compile]
emit = "binary"
crate_type = "bin"
edition = "2021"
"#,
    )
    .expect("write cista manifest");
    fs::write(
        source.join("rust/Cargo.toml"),
        "[package]\nname = \"tool\"\nversion = \"1.2.3\"\nedition = \"2021\"\n",
    )
    .expect("write cargo manifest");
    fs::write(source.join("rust/src/main.rs"), "fn main() {}\n").expect("write rust source");

    publish(&source, Path::new("cista.toml"), Some(&registry)).expect("publish snapshot");
    fs::remove_dir_all(&source).expect("remove original source");
    let fetched =
        fetch_to_cache("tool@1.2.3", Some(&registry), Some(&store)).expect("fetch exact package");
    assert!(fetched.join("cista.toml").is_file());
    assert!(fetched.join("rust/src/main.rs").is_file());
    assert!(publish(&fetched, Path::new("cista.toml"), Some(&registry)).is_err());
    assert!(fetch_to_cache("tool", Some(&registry), Some(&store)).is_err());
    assert!(fetch_to_cache("../tool@1.2.3", Some(&registry), Some(&store)).is_err());

    fs::remove_dir_all(root).expect("cleanup temp root");
}
