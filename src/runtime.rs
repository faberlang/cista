//! Target-native runtime binding metadata.

use crate::package::PackageId;

/// Runtime binding identity for a package and target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeBinding {
    /// Package providing the Faber interface.
    pub package: PackageId,

    /// Target identifier such as `rust`.
    pub target: String,
}

impl RuntimeBinding {
    /// Create a runtime binding identity.
    pub fn new(package: PackageId, target: impl Into<String>) -> Self {
        Self {
            package,
            target: target.into(),
        }
    }
}
