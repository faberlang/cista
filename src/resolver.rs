//! Package and provider resolution.

use crate::package::PackageId;

/// Unresolved package request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageRequest {
    /// Requested package.
    pub package: PackageId,
}

impl PackageRequest {
    /// Create a package request.
    pub fn new(package: PackageId) -> Self {
        Self { package }
    }
}
