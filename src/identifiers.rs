use crate::identifiers::generated::GeneratedIdentifier;

mod generated;

/// A generated identifier with an associated human-readable name.
#[derive(Debug, Clone)]
pub struct NamedId {
    name: String,
    id: GeneratedIdentifier,
}

impl NamedId {
    /// Create a new `NamedId` with the given name and a random ID.
    pub fn new(name: String) -> Self {
        let id = GeneratedIdentifier::new();
        Self { name, id }
    }
}

impl Default for NamedId {
    /// Creates a `NamedId` with the name "unnamed"
    fn default() -> Self {
        Self::new("unnamed".to_string())
    }
}

impl std::fmt::Display for NamedId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}
