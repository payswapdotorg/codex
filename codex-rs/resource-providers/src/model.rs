//! The provider-neutral resource model (WO-016).
//!
//! This module owns the vocabulary every resource provider exchanges with
//! the host: provider identity, resource classes, validated resource
//! specs, credential-free resource handles, lifecycle statuses, network
//! policy inputs, and snapshot artifacts. The vocabulary mirrors the
//! payload discipline of the WO-003/WO-005 contract crates: serde
//! camelCase with `deny_unknown_fields`, bounded sizes, and validation at
//! construction so malformed records never reach execution.
//!
//! Nothing here is workflow semantics: a [`ResourceHandle`] names one
//! provider-owned infrastructure instance and its resource-plane
//! lifecycle status. Workflow meaning stays in `codex-workflow-contracts`;
//! resource lifecycle state never becomes workflow semantic state.

use std::fmt;

pub use codex_execution_contracts::ResourceId;
use serde::Deserialize;
use serde::Serialize;

use crate::ResourceProviderError;

/// Maximum serialized size of an `os` or `region` string, in bytes.
pub const SPEC_TEXT_MAX_BYTES: usize = 64;
/// Maximum number of labels on one resource spec.
pub const SPEC_LABELS_MAX: usize = 16;
/// Maximum length of one label key, in bytes.
pub const LABEL_KEY_MAX_BYTES: usize = 64;
/// Maximum length of one label value, in bytes.
pub const LABEL_VALUE_MAX_BYTES: usize = 128;
/// Maximum CPU allocation request, in milli-CPU (one thousand vCPU).
pub const CPU_MILLIS_MAX: u32 = 1_000_000;
/// Maximum memory allocation request, in MiB (one TiB).
pub const MEMORY_MIB_MAX: u32 = 1_048_576;
/// Maximum disk allocation request, in MiB (one TiB).
pub const DISK_MIB_MAX: u64 = 1_048_576;
/// Maximum GPU device count request.
pub const GPU_MAX: u32 = 64;
/// Maximum hosts listed in one restricted network policy.
pub const NETWORK_HOSTS_MAX: usize = 16;
/// Maximum length of one network host token, in bytes.
pub const NETWORK_HOST_MAX_BYTES: usize = 253;
/// Maximum length of a snapshot artifact reference, in bytes.
pub const SNAPSHOT_ARTIFACT_MAX_BYTES: usize = 128;

/// Identity of one execution resource provider (for example
/// `fixture-sandbox-cloud` or a real host-supplied provider id).
///
/// Provider ids use path-segment-safe tokens — the same rules as the
/// WO-005 `AdapterId`/`ResourceId` newtypes — so they can appear in logs,
/// evidence locators, and debug surfaces without escaping. Provider ids
/// never enter workflow semantics: they scope provider-owned evidence and
/// registry wiring only.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProviderId(String);

impl ProviderId {
    /// Parses a provider identifier.
    ///
    /// Follows the Codex plugin/skill naming rules: non-empty ASCII
    /// letters, digits, `_`, `-`, and `.` separating non-empty segments.
    pub fn parse(value: impl Into<String>) -> Result<Self, ResourceProviderError> {
        let value = value.into();
        validate_name_segment(&value, "provider id")?;
        Ok(Self(value))
    }
}

impl TryFrom<String> for ProviderId {
    type Error = ResourceProviderError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for ProviderId {
    type Error = ResourceProviderError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<ProviderId> for String {
    fn from(value: ProviderId) -> Self {
        value.0
    }
}

impl AsRef<str> for ProviderId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for ProviderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

/// The class of execution resource a provider provisions.
///
/// Classes are provider-neutral buckets of infrastructure capability:
/// E2B-shaped sandbox clouds, Kubernetes clusters, local container
/// runtimes, and VM farms can all serve `Sandbox`; only the provider's
/// descriptor says which classes it actually serves. Workflow semantics
/// never reference classes — the workflow plane sees only logical
/// resource types (see [`crate::bind`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceClass {
    /// An isolated, typically ephemeral compute sandbox.
    Sandbox,
    /// A remote graphical desktop session, reached through the existing
    /// Computer Use semantic contract as a resource binding beneath it.
    RemoteDesktop,
    /// A persistent workspace whose state survives across sessions.
    PersistentWorkspace,
    /// Specialized compute with attached GPU devices.
    GpuCompute,
}

impl ResourceClass {
    /// Every resource class defined by the provider plane.
    pub const ALL: [Self; 4] = [
        Self::Sandbox,
        Self::RemoteDesktop,
        Self::PersistentWorkspace,
        Self::GpuCompute,
    ];

    /// The canonical wire name, matching the serde form.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sandbox => "sandbox",
            Self::RemoteDesktop => "remoteDesktop",
            Self::PersistentWorkspace => "persistentWorkspace",
            Self::GpuCompute => "gpuCompute",
        }
    }

    /// Parses a wire name into a resource class.
    ///
    /// Only canonical wire names are accepted, so the parsed form always
    /// round-trips through serde unchanged.
    pub fn parse(value: impl AsRef<str>) -> Result<Self, ResourceProviderError> {
        let value = value.as_ref();
        for class in Self::ALL {
            if class.as_str() == value {
                return Ok(class);
            }
        }
        Err(ResourceProviderError::InvalidSpec {
            reason: format!("unknown resource class `{value}`"),
        })
    }

    /// The canonical logical resource type name this class binds under
    /// (see [`crate::bind`]).
    pub const fn logical_resource_type(self) -> &'static str {
        match self {
            Self::Sandbox => crate::bind::SANDBOX_COMPUTE_RESOURCE_TYPE,
            Self::RemoteDesktop => crate::bind::REMOTE_DESKTOP_RESOURCE_TYPE,
            Self::PersistentWorkspace => crate::bind::PERSISTENT_WORKSPACE_RESOURCE_TYPE,
            Self::GpuCompute => crate::bind::GPU_COMPUTE_RESOURCE_TYPE,
        }
    }
}

impl fmt::Display for ResourceClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The resource-plane lifecycle status of one resource instance.
///
/// This is provider/resource state only. It never becomes workflow
/// semantic state: a workflow instance's status lives in the WO-003
/// instance contracts, and the control plane owns durable transitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceStatus {
    /// The resource is being provisioned and not usable yet.
    Provisioning,
    /// The resource is live and usable.
    Active,
    /// The resource is paused; state is preserved where the provider
    /// supports pause/resume.
    Paused,
    /// The resource was explicitly released; the instance no longer runs.
    Released,
    /// The resource was lost by the provider (eviction, crash, expiry).
    Lost,
}

impl ResourceStatus {
    /// Every resource status defined by the provider plane.
    pub const ALL: [Self; 5] = [
        Self::Provisioning,
        Self::Active,
        Self::Paused,
        Self::Released,
        Self::Lost,
    ];

    /// The canonical wire name, matching the serde form.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Provisioning => "provisioning",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Released => "released",
            Self::Lost => "lost",
        }
    }

    /// Whether a resource in this status may be bound to a workflow
    /// requirement through [`crate::bind`].
    ///
    /// Only live resources mint bindings: provisioning resources do not
    /// exist yet, and released/lost resources are gone.
    pub const fn is_bindable(self) -> bool {
        matches!(self, Self::Active | Self::Paused)
    }
}

impl fmt::Display for ResourceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A provider-neutral network policy input for one resource.
///
/// Network policy is a *request* expressed in the spec; a provider only
/// honors it when its capability declaration flags network policy
/// support, and a spec carrying a policy against an undeclared provider
/// is rejected explicitly at create time.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum NetworkPolicy {
    /// No outbound network access.
    Isolated,
    /// Outbound access restricted to the listed hosts.
    Restricted {
        /// The hosts the resource may reach.
        allowed_hosts: Vec<String>,
    },
    /// Outbound access unrestricted.
    Open,
}

/// A validated request to provision one resource.
///
/// Every field beyond `class` is optional; absent fields mean the
/// provider's defaults apply. All fields are bounded so a malformed
/// request cannot destabilize the runtime, and specs are credential-free
/// by construction: they carry infrastructure shape, never secret
/// material.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceSpec {
    /// The class of resource being requested.
    pub class: ResourceClass,
    /// Requested CPU allocation in milli-CPU, when constrained.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_millis: Option<u32>,
    /// Requested memory allocation in MiB, when constrained.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_mib: Option<u32>,
    /// Requested disk allocation in MiB, when constrained.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk_mib: Option<u64>,
    /// Requested GPU device count, when GPU compute is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu: Option<u32>,
    /// Operating system or runtime identifier (for example `linux`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    /// Provider region the resource should live in, when constrained.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// Free-form infrastructure labels, bounded in count and size.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub labels: std::collections::BTreeMap<String, String>,
    /// Network policy input, honored only by providers that declare
    /// network-policy support.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkPolicy>,
}

impl ResourceSpec {
    /// Validates the spec's bounds.
    ///
    /// - capacity requests are within the provider-plane bounds;
    /// - `os`/`region` are bounded ASCII without control characters;
    /// - labels are bounded in count, key, and value size;
    /// - restricted network policies list bounded, duplicate-free hosts.
    pub fn validate(&self) -> Result<(), ResourceProviderError> {
        if let Some(cpu) = self.cpu_millis
            && (cpu == 0 || cpu > CPU_MILLIS_MAX)
        {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!("cpuMillis must be 1..={CPU_MILLIS_MAX}"),
            });
        }
        if let Some(memory) = self.memory_mib
            && (memory == 0 || memory > MEMORY_MIB_MAX)
        {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!("memoryMiB must be 1..={MEMORY_MIB_MAX}"),
            });
        }
        if let Some(disk) = self.disk_mib
            && (disk == 0 || disk > DISK_MIB_MAX)
        {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!("diskMiB must be 1..={DISK_MIB_MAX}"),
            });
        }
        if let Some(gpu) = self.gpu
            && (gpu == 0 || gpu > GPU_MAX)
        {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!("gpu must be 1..={GPU_MAX}"),
            });
        }
        validate_text(&self.os, "os")?;
        validate_text(&self.region, "region")?;
        if self.labels.len() > SPEC_LABELS_MAX {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!("specs carry at most {SPEC_LABELS_MAX} labels"),
            });
        }
        for (key, value) in &self.labels {
            if key.is_empty() || key.len() > LABEL_KEY_MAX_BYTES {
                return Err(ResourceProviderError::InvalidSpec {
                    reason: format!("label keys must be 1..={LABEL_KEY_MAX_BYTES} bytes"),
                });
            }
            if value.len() > LABEL_VALUE_MAX_BYTES {
                return Err(ResourceProviderError::InvalidSpec {
                    reason: format!("label values must be at most {LABEL_VALUE_MAX_BYTES} bytes"),
                });
            }
        }
        if let Some(NetworkPolicy::Restricted { allowed_hosts }) = &self.network {
            if allowed_hosts.len() > NETWORK_HOSTS_MAX {
                return Err(ResourceProviderError::InvalidSpec {
                    reason: format!("network policies list at most {NETWORK_HOSTS_MAX} hosts"),
                });
            }
            let mut seen = std::collections::BTreeSet::new();
            for host in allowed_hosts {
                if host.is_empty() || host.len() > NETWORK_HOST_MAX_BYTES {
                    return Err(ResourceProviderError::InvalidSpec {
                        reason: format!("network hosts must be 1..={NETWORK_HOST_MAX_BYTES} bytes"),
                    });
                }
                if host.chars().any(char::is_control) {
                    return Err(ResourceProviderError::InvalidSpec {
                        reason: "network hosts must not contain control characters".to_owned(),
                    });
                }
                if !seen.insert(host) {
                    return Err(ResourceProviderError::InvalidSpec {
                        reason: format!("network host `{host}` is listed twice"),
                    });
                }
            }
        }
        Ok(())
    }
}

/// One concrete, credential-free resource instance owned by a provider.
///
/// The handle names the instance by its opaque [`ResourceId`] (minted by
/// the provider, interpreted only by the provider), the class it was
/// provisioned as, the owning provider, and its current resource-plane
/// status. Credentials never appear in handles: secret material stays
/// with the host's secret store and the provider's own runtime.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceHandle {
    /// The opaque resource instance identity.
    pub resource: ResourceId,
    /// The class the resource was provisioned as.
    pub class: ResourceClass,
    /// The provider that owns and interprets the instance.
    pub provider: ProviderId,
    /// The resource's current lifecycle status.
    pub status: ResourceStatus,
}

/// A snapshot/checkpoint captured from one resource.
///
/// Snapshots are **execution artifacts**: they name provider-side state
/// that can speed up recovery or forking, and they never replace
/// immutable workflow/version/source/dependency identity. The artifact
/// reference is opaque to this crate.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceSnapshot {
    /// The resource the snapshot was captured from.
    pub resource: ResourceId,
    /// The opaque snapshot artifact reference.
    pub artifact: String,
}

impl ResourceSnapshot {
    /// Validates the snapshot record's bounds.
    pub fn validate(&self) -> Result<(), ResourceProviderError> {
        if self.artifact.is_empty() || self.artifact.len() > SNAPSHOT_ARTIFACT_MAX_BYTES {
            return Err(ResourceProviderError::InvalidSpec {
                reason: format!(
                    "snapshot artifacts must be 1..={SNAPSHOT_ARTIFACT_MAX_BYTES} bytes"
                ),
            });
        }
        if self.artifact.chars().any(char::is_control) {
            return Err(ResourceProviderError::InvalidSpec {
                reason: "snapshot artifacts must not contain control characters".to_owned(),
            });
        }
        Ok(())
    }
}

/// Validates path-segment-safe name tokens shared by provider ids,
/// mirroring the WO-005 `AdapterId`/`ResourceId` rules.
fn validate_name_segment(value: &str, kind: &'static str) -> Result<(), ResourceProviderError> {
    let characters_ok = !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        });
    let dots_ok = !value.starts_with('.') && !value.ends_with('.') && !value.contains("..");
    if characters_ok && dots_ok {
        Ok(())
    } else {
        Err(ResourceProviderError::InvalidIdentifier {
            kind,
            value: value.to_owned(),
            reason: "must be a non-empty path-segment-safe token",
        })
    }
}

/// Validates a bounded, control-free ASCII text field.
fn validate_text(value: &Option<String>, field: &'static str) -> Result<(), ResourceProviderError> {
    if let Some(text) = value
        && (text.is_empty()
            || text.len() > SPEC_TEXT_MAX_BYTES
            || text.chars().any(|character| !character.is_ascii_graphic()))
    {
        return Err(ResourceProviderError::InvalidSpec {
            reason: format!("{field} must be non-empty ASCII, at most {SPEC_TEXT_MAX_BYTES} bytes"),
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
