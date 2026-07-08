//! Package target metadata.

/// Target identifier known to package metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetId {
    /// Target name, such as `rust`.
    pub name: String,
}

impl TargetId {
    /// Create a target identifier.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}
