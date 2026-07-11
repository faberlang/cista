use std::path::Path;

use super::is_interface_file;

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
