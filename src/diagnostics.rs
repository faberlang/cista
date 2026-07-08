//! Diagnostics for package-store operations.

/// Package-layer diagnostic record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// Human-readable diagnostic message.
    pub message: String,
}

impl Diagnostic {
    /// Create a diagnostic from a message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
