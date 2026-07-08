//! Package identity and source layout.

/// Package identifier.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PackageId {
    /// Canonical package name.
    pub name: String,
}

impl PackageId {
    /// Create a package identifier.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}
