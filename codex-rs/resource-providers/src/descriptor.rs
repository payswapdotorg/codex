//! Provider capability declarations (WO-016).
//!
//! A [`ProviderDescriptor`] is a provider's *declaration* of what it
//! serves: which resource classes, which lifecycle operations, capacity
//! ceilings, declared health, regions, and cost metadata. Declaration is
//! discovery data only — **capability does not imply authorization**.
//! Binding and authorization decisions stay with the WO-005
//! `CapabilityRegistry` and the approval plane; a provider can never
//! escalate itself into a workflow by declaring more.
//!
//! Descriptors are validated at registration so a malformed declaration
//! is rejected before it can mislead resource selection.

use serde::Deserialize;
use serde::Serialize;

use crate::ResourceProviderError;
use crate::model::CPU_MILLIS_MAX;
use crate::model::DISK_MIB_MAX;
use crate::model::MEMORY_MIB_MAX;
use crate::model::ProviderId;
use crate::model::ResourceClass;
use crate::provider::LifecycleOp;

/// Maximum regions declared by one provider.
pub const PROVIDER_REGIONS_MAX: usize = 8;
/// Maximum length of a region token, in bytes.
pub const PROVIDER_REGION_MAX_BYTES: usize = 32;
/// Maximum length of a health detail message, in bytes.
pub const PROVIDER_HEALTH_DETAIL_MAX_BYTES: usize = 256;
/// Maximum length of a cost currency token, in bytes.
pub const COST_CURRENCY_MAX_BYTES: usize = 16;

/// Which lifecycle operations and request features a provider supports
/// for one resource class.
///
/// Flags are declaration-only: an undeclared operation stays explicitly
/// unsupported (a [`ResourceProviderError::Unsupported`] error), and a
/// spec feature the provider does not declare (network policy, GPU) is
/// rejected at create time rather than silently ignored.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceLifecycleSupport {
    /// The provider can provision fresh resources of the class.
    pub create: bool,
    /// The provider can pause and resume resources of the class.
    pub pause_resume: bool,
    /// The provider can capture snapshot/checkpoint artifacts.
    pub snapshot: bool,
    /// The provider can fork a resource's state into a new resource.
    pub fork: bool,
    /// The provider can resize a live resource.
    pub resize: bool,
    /// The provider honors network policy inputs in specs.
    pub network_policy: bool,
    /// The provider can serve GPU-bearing specs.
    pub gpu: bool,
}

impl ResourceLifecycleSupport {
    /// A declaration with every flag off.
    pub fn none() -> Self {
        Self::default()
    }

    /// Declares create support.
    pub fn with_create(mut self) -> Self {
        self.create = true;
        self
    }

    /// Declares pause/resume support.
    pub fn with_pause_resume(mut self) -> Self {
        self.pause_resume = true;
        self
    }

    /// Declares snapshot support.
    pub fn with_snapshot(mut self) -> Self {
        self.snapshot = true;
        self
    }

    /// Declares fork support.
    pub fn with_fork(mut self) -> Self {
        self.fork = true;
        self
    }

    /// Declares resize support.
    pub fn with_resize(mut self) -> Self {
        self.resize = true;
        self
    }

    /// Declares network-policy support.
    pub fn with_network_policy(mut self) -> Self {
        self.network_policy = true;
        self
    }

    /// Declares GPU support.
    pub fn with_gpu(mut self) -> Self {
        self.gpu = true;
        self
    }

    /// Whether this declaration supports the lifecycle `op`.
    pub fn supports(self, op: &LifecycleOp) -> bool {
        match op {
            LifecycleOp::Create => self.create,
            LifecycleOp::Pause | LifecycleOp::Resume => self.pause_resume,
            LifecycleOp::Snapshot => self.snapshot,
            LifecycleOp::Fork => self.fork,
            LifecycleOp::Resize(_) => self.resize,
        }
    }
}

/// The capacity ceiling one provider declares for one resource class.
///
/// Quotas are advisory ceilings for selection and validation; they are
/// not authorization and not a reservation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceQuota {
    /// Maximum CPU allocation in milli-CPU.
    pub max_cpu_millis: u32,
    /// Maximum memory allocation in MiB.
    pub max_memory_mib: u32,
    /// Maximum disk allocation in MiB.
    pub max_disk_mib: u64,
}

impl ResourceQuota {
    /// Validates the quota's bounds.
    ///
    /// Quotas must be non-zero and within the provider-plane request
    /// bounds, so a declared ceiling can always actually be requested.
    pub fn validate(self) -> Result<(), ResourceProviderError> {
        if self.max_cpu_millis == 0 || self.max_cpu_millis > CPU_MILLIS_MAX {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!("quota maxCpuMillis must be 1..={CPU_MILLIS_MAX}"),
            });
        }
        if self.max_memory_mib == 0 || self.max_memory_mib > MEMORY_MIB_MAX {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!("quota maxMemoryMiB must be 1..={MEMORY_MIB_MAX}"),
            });
        }
        if self.max_disk_mib == 0 || self.max_disk_mib > DISK_MIB_MAX {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!("quota maxDiskMiB must be 1..={DISK_MIB_MAX}"),
            });
        }
        Ok(())
    }
}

/// One resource class a provider serves, with its declared lifecycle
/// support and capacity ceiling.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceCapability {
    /// The resource class being served.
    pub class: ResourceClass,
    /// The lifecycle operations and features declared for the class.
    pub lifecycle: ResourceLifecycleSupport,
    /// The capacity ceiling declared for the class.
    pub capacity: ResourceQuota,
}

impl ResourceCapability {
    /// Validates the capability record.
    pub fn validate(&self) -> Result<(), ResourceProviderError> {
        self.capacity.validate()
    }
}

/// The health status a provider reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderHealthStatus {
    /// The provider is serving requests.
    Healthy,
    /// The provider is serving with degraded capacity or latency.
    Degraded,
    /// The provider cannot serve requests right now.
    Unreachable,
}

impl ProviderHealthStatus {
    /// Every health status defined by the provider plane.
    pub const ALL: [Self; 3] = [Self::Healthy, Self::Degraded, Self::Unreachable];

    /// The canonical wire name, matching the serde form.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unreachable => "unreachable",
        }
    }

    /// Parses a wire name into a health status.
    pub fn parse(value: impl AsRef<str>) -> Result<Self, ResourceProviderError> {
        let value = value.as_ref();
        for status in Self::ALL {
            if status.as_str() == value {
                return Ok(status);
            }
        }
        Err(ResourceProviderError::InvalidSpec {
            reason: format!("unknown provider health status `{value}`"),
        })
    }
}

/// A provider health answer.
///
/// Health is a probe result, never an authorization: a healthy provider
/// still crosses the binding policy and approval gates before any of its
/// resources execute workflow work.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderHealth {
    /// The reported health status.
    pub status: ProviderHealthStatus,
    /// Bounded human-facing detail, when the provider has any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Round-trip probe latency hint in milliseconds, when measured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

impl ProviderHealth {
    /// A healthy answer with no detail.
    pub fn healthy() -> Self {
        Self {
            status: ProviderHealthStatus::Healthy,
            detail: None,
            latency_ms: None,
        }
    }

    /// Validates the health record's bounds.
    pub fn validate(&self) -> Result<(), ResourceProviderError> {
        if let Some(detail) = &self.detail
            && (detail.is_empty() || detail.len() > PROVIDER_HEALTH_DETAIL_MAX_BYTES)
        {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!(
                    "health details must be 1..={PROVIDER_HEALTH_DETAIL_MAX_BYTES} bytes"
                ),
            });
        }
        Ok(())
    }
}

/// Cost metadata a provider declares for its resources.
///
/// Cost is decision-support data for hosts and operators; it never
/// becomes workflow semantics and never carries payment credentials.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CostMetadata {
    /// The cost currency (for example `usd`).
    pub currency: String,
    /// The cost per resource-hour in millionths of the currency unit.
    pub per_hour_micros: u64,
}

impl CostMetadata {
    /// Validates the cost record's bounds.
    pub fn validate(&self) -> Result<(), ResourceProviderError> {
        if self.currency.is_empty()
            || self.currency.len() > COST_CURRENCY_MAX_BYTES
            || self
                .currency
                .chars()
                .any(|character| !character.is_ascii_lowercase() && !character.is_ascii_digit())
        {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!(
                    "cost currency must be lowercase ASCII, at most {COST_CURRENCY_MAX_BYTES} \
                     bytes"
                ),
            });
        }
        Ok(())
    }
}

/// The immutable registration descriptor of one resource provider.
///
/// The descriptor is the provider's declaration of what it serves; the
/// host and the registries own whether and how those declarations become
/// bindings. `health` is the declared baseline snapshot —
/// [`crate::ExecutionResourceProvider::probe`] answers live health.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderDescriptor {
    /// The provider's identity.
    pub provider: ProviderId,
    /// The resource classes the provider serves, one capability each.
    pub capabilities: Vec<ResourceCapability>,
    /// The provider's declared baseline health.
    pub health: ProviderHealth,
    /// The regions the provider serves.
    pub regions: Vec<String>,
    /// Cost metadata, when the provider charges for resources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostMetadata>,
}

impl ProviderDescriptor {
    /// Validates the descriptor's structural rules.
    ///
    /// - at least one capability is declared;
    /// - resource classes are unique;
    /// - every capability and its quota is well-formed;
    /// - the declared health is bounded;
    /// - regions are declared, unique, and bounded;
    /// - cost metadata, when present, is well-formed.
    pub fn validate(&self) -> Result<(), ResourceProviderError> {
        if self.capabilities.is_empty() {
            return Err(ResourceProviderError::InvalidSpec {
                reason: "provider descriptors declare at least one capability".to_owned(),
            });
        }
        let mut seen = std::collections::BTreeSet::new();
        for capability in &self.capabilities {
            capability.validate()?;
            if !seen.insert(capability.class) {
                return Err(ResourceProviderError::InvalidSpec {
                    reason: format!(
                        "provider `{}` declares class `{}` twice",
                        self.provider, capability.class
                    ),
                });
            }
        }
        self.health.validate()?;
        if self.regions.is_empty() || self.regions.len() > PROVIDER_REGIONS_MAX {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!("providers declare 1..={PROVIDER_REGIONS_MAX} regions"),
            });
        }
        let mut regions = std::collections::BTreeSet::new();
        for region in &self.regions {
            if region.is_empty()
                || region.len() > PROVIDER_REGION_MAX_BYTES
                || region.chars().any(char::is_control)
            {
                return Err(ResourceProviderError::InvalidSpec {
                    reason: format!(
                        "regions must be 1..={PROVIDER_REGION_MAX_BYTES} bytes without \
                         control characters"
                    ),
                });
            }
            if !regions.insert(region) {
                return Err(ResourceProviderError::InvalidSpec {
                    reason: format!("region `{region}` is declared twice"),
                });
            }
        }
        if let Some(cost) = &self.cost {
            cost.validate()?;
        }
        Ok(())
    }

    /// The capability declared for `class`, when the provider serves it.
    pub fn capability_for(&self, class: ResourceClass) -> Option<&ResourceCapability> {
        self.capabilities
            .iter()
            .find(|capability| capability.class == class)
    }

    /// Whether the provider declares serving `class`.
    pub fn serves(&self, class: ResourceClass) -> bool {
        self.capability_for(class).is_some()
    }
}

#[cfg(test)]
#[path = "descriptor_tests.rs"]
mod tests;
