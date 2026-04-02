//! Chrome transport capability policy models and conversions.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeCapabilityPolicy {
    Allow,
    Deny,
}

impl Default for ChromeCapabilityPolicy {
    fn default() -> Self {
        Self::Allow
    }
}

impl From<cbf::data::policy::CapabilityPolicy> for ChromeCapabilityPolicy {
    fn from(value: cbf::data::policy::CapabilityPolicy) -> Self {
        match value {
            cbf::data::policy::CapabilityPolicy::Allow => Self::Allow,
            cbf::data::policy::CapabilityPolicy::Deny => Self::Deny,
        }
    }
}

impl From<ChromeCapabilityPolicy> for cbf::data::policy::CapabilityPolicy {
    fn from(value: ChromeCapabilityPolicy) -> Self {
        match value {
            ChromeCapabilityPolicy::Allow => Self::Allow,
            ChromeCapabilityPolicy::Deny => Self::Deny,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChromeIpcPolicy {
    Deny,
    Allow { allowed_origins: Vec<String> },
}

impl Default for ChromeIpcPolicy {
    fn default() -> Self {
        Self::Deny
    }
}

impl From<cbf::data::policy::IpcPolicy> for ChromeIpcPolicy {
    fn from(value: cbf::data::policy::IpcPolicy) -> Self {
        match value {
            cbf::data::policy::IpcPolicy::Deny => Self::Deny,
            cbf::data::policy::IpcPolicy::Allow { allowed_origins } => {
                Self::Allow { allowed_origins }
            }
        }
    }
}

impl From<ChromeIpcPolicy> for cbf::data::policy::IpcPolicy {
    fn from(value: ChromeIpcPolicy) -> Self {
        match value {
            ChromeIpcPolicy::Deny => Self::Deny,
            ChromeIpcPolicy::Allow { allowed_origins } => Self::Allow { allowed_origins },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChromeBrowsingContextPolicy {
    pub ipc: ChromeIpcPolicy,
    pub extensions: ChromeCapabilityPolicy,
}

impl From<cbf::data::policy::BrowsingContextPolicy> for ChromeBrowsingContextPolicy {
    fn from(value: cbf::data::policy::BrowsingContextPolicy) -> Self {
        Self {
            ipc: value.ipc.into(),
            extensions: value.extensions.into(),
        }
    }
}

impl From<ChromeBrowsingContextPolicy> for cbf::data::policy::BrowsingContextPolicy {
    fn from(value: ChromeBrowsingContextPolicy) -> Self {
        Self {
            ipc: value.ipc.into(),
            extensions: value.extensions.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ChromeBrowsingContextPolicy, ChromeCapabilityPolicy, ChromeIpcPolicy};

    #[test]
    fn browsing_context_policy_round_trip_with_generic() {
        let policy = ChromeBrowsingContextPolicy {
            ipc: ChromeIpcPolicy::Allow {
                allowed_origins: vec!["app://simpleapp".to_string()],
            },
            extensions: ChromeCapabilityPolicy::Deny,
        };

        let generic: cbf::data::policy::BrowsingContextPolicy = policy.clone().into();
        let round_trip = ChromeBrowsingContextPolicy::from(generic);

        assert_eq!(round_trip, policy);
    }
}
