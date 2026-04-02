//! Browser-generic browsing context capability policy models.

/// Allow or deny a capability for a browsing context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityPolicy {
    Allow,
    Deny,
}

impl Default for CapabilityPolicy {
    fn default() -> Self {
        Self::Allow
    }
}

/// IPC policy for a browsing context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcPolicy {
    Deny,
    Allow { allowed_origins: Vec<String> },
}

impl Default for IpcPolicy {
    fn default() -> Self {
        Self::Deny
    }
}

/// Create-time capability policy for a browsing context.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BrowsingContextPolicy {
    pub ipc: IpcPolicy,
    pub extensions: CapabilityPolicy,
}

#[cfg(test)]
mod tests {
    use super::{BrowsingContextPolicy, CapabilityPolicy, IpcPolicy};

    #[test]
    fn default_policy_matches_legacy_defaults() {
        assert_eq!(
            BrowsingContextPolicy::default(),
            BrowsingContextPolicy {
                ipc: IpcPolicy::Deny,
                extensions: CapabilityPolicy::Allow,
            }
        );
    }
}
