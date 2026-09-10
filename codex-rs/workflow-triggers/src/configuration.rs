//! Installed workflow configuration: the explicit, reviewable bindings
//! that make an immutable version operationally usable.
//!
//! Installation never touches workflow semantics: the version record is
//! sealed and immutable. What installation owns is *configuration* — which
//! resources and accounts serve the version's declared requirements, which
//! dependency implementations satisfy its lock, which trigger classes and
//! routing endpoints may start it, and which schedules poll for it. Every
//! later change to this record (a resource rebind) is an audited
//! control-plane transition that provably leaves the semantic source
//! untouched.

use codex_execution_contracts::BindingPolicy;
use codex_execution_contracts::ResourceBinding;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::ResourceTypeId;
use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowVersionId;
use codex_workflow_forge::PublishedVersionRef;
use serde::Deserialize;
use serde::Serialize;

use crate::ResourceAuthorization;
use crate::ScheduleSpec;

/// One registered trigger route on an installation.
///
/// The binding is the *only* way a trigger class reaches an installed
/// workflow: a class that is not bound here cannot start it. Endpoint
/// routing keeps externally-delivered events (webhook paths, connector
/// subscriptions) explicit instead of implicit.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TriggerBinding {
    /// The trigger class this binding accepts.
    pub trigger: TriggerClass,
    /// The opaque routing endpoint the event must arrive on (webhook
    /// path, connector subscription id, session label). `None` accepts
    /// directly-addressed events of this class (user starts, schedule
    /// fires).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

impl TriggerBinding {
    /// Builds a direct (non-endpoint) trigger binding.
    pub fn direct(trigger: TriggerClass) -> Self {
        Self {
            trigger,
            endpoint: None,
        }
    }

    /// Builds an endpoint-routed trigger binding.
    pub fn routed(trigger: TriggerClass, endpoint: impl Into<String>) -> Self {
        Self {
            trigger,
            endpoint: Some(endpoint.into()),
        }
    }

    /// Whether an event of `class` arriving on `endpoint` matches this
    /// binding (strict endpoint equality).
    pub fn matches(&self, class: TriggerClass, endpoint: Option<&str>) -> bool {
        self.trigger == class && self.endpoint.as_deref() == endpoint
    }
}

/// One explicitly bound dependency implementation of an installed
/// workflow.
///
/// The binding resolves a locked dependency to a concrete host-side
/// implementation (an installed skill, plugin bundle, or MCP server
/// identity). Its integrity digest must equal the dependency-lock entry
/// of the installed version, which is what makes silent dependency
/// upgrades structurally impossible: a drifted binding is refused at
/// install time and at fire time.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DependencyBinding {
    /// The declared dependency key in its compact string form (see
    /// `DependencyKey::as_key`, for example `skill:browser-use`).
    pub key: String,
    /// Opaque identity of the resolved implementation (never a
    /// credential).
    pub binding: String,
    /// The integrity digest of the resolved implementation as recorded in
    /// the version's dependency lock; `None` mirrors a lock entry that
    /// records no integrity digest.
    pub integrity: Option<ContentDigest>,
}

/// One registered schedule on an installation.
///
/// The cursor records the poll instant up to which occurrences have been
/// consumed; it only ever moves forward.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleRegistration {
    /// The schedule's identity, unique within the installation.
    pub schedule_id: String,
    /// The firing specification.
    pub spec: ScheduleSpec,
    /// The registration instant the occurrence arithmetic anchors to.
    pub anchor_unix_ms: u64,
    /// The poll instant up to which occurrences have been consumed.
    pub cursor_unix_ms: u64,
}

/// The audited record of one resource or account rebind.
///
/// Rebinds are configuration changes: they prove the immutable semantic
/// source was untouched by carrying the from/to resource identities and
/// the instant of the change.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RebindRecord {
    /// The workflow whose configuration changed.
    pub workflow: WorkflowDefinitionId,
    /// The rebound resource type.
    pub resource_type: ResourceTypeId,
    /// The resource instance previously bound, when one was.
    pub from: Option<String>,
    /// The resource instance now bound.
    pub to: String,
    /// When the rebind was recorded (unix milliseconds).
    pub at_unix_ms: u64,
}

/// The installed configuration of one workflow.
///
/// This record is durable host state crossing the
/// [`crate::InstallationStore`] seam. It pins an immutable published
/// version (by identity and forge reference) and holds every explicit
/// binding the version needs to run: resources, dependencies, triggers,
/// and schedules.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstalledConfiguration {
    /// The installed workflow.
    pub workflow: WorkflowDefinitionId,
    /// The immutable version the configuration pins.
    pub version_id: WorkflowVersionId,
    /// The forge install reference for the pinned version.
    pub reference: PublishedVersionRef,
    /// The binding policy applied when instances start.
    pub policy: BindingPolicy,
    /// Explicit resource and account bindings, credential-free.
    pub resources: Vec<ResourceBinding>,
    /// Explicit dependency implementation bindings.
    pub dependencies: Vec<DependencyBinding>,
    /// The trigger classes and routing endpoints that may start the
    /// workflow.
    pub trigger_bindings: Vec<TriggerBinding>,
    /// The registered schedules.
    pub schedules: Vec<ScheduleRegistration>,
    /// The resource authorizations recorded at install time (audit; each
    /// fire re-checks authorization freshly).
    pub authorizations: Vec<ResourceAuthorization>,
    /// The walk budget for started instances.
    pub max_walk_steps: u64,
    /// When the configuration was installed (unix milliseconds).
    pub installed_at_unix_ms: u64,
}

impl InstalledConfiguration {
    /// The binding for one resource type, when configured.
    pub fn resource_binding(&self, resource_type: &ResourceTypeId) -> Option<&ResourceBinding> {
        self.resources
            .iter()
            .find(|binding| &binding.resource_type == resource_type)
    }

    /// The trigger binding matching an event class and routing endpoint.
    pub fn trigger_binding_matching(
        &self,
        class: TriggerClass,
        endpoint: Option<&str>,
    ) -> Option<&TriggerBinding> {
        self.trigger_bindings
            .iter()
            .find(|binding| binding.matches(class, endpoint))
    }
}
