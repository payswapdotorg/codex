//! Shared test support for the durable store unit tests.
//!
//! Provides scratch root directories (unique per test, best-effort
//! cleanup) and the light record builders the tests drive the ports
//! with — the same shapes the in-memory doubles' tests use.

use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use codex_workflow_contracts::DependencyLock;
use codex_workflow_contracts::DevelopmentRef;
use codex_workflow_contracts::IR_FORMAT_VERSION;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::ResourceTypeId;
use codex_workflow_contracts::RevisionSha;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::StepNode;
use codex_workflow_contracts::WorkflowDefinition;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowInstance;
use codex_workflow_contracts::WorkflowInstanceId;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;
use codex_workflow_forge::PublishedVersionRef;
use codex_workflow_triggers::InstalledConfiguration;
use codex_workflow_triggers::RebindRecord;

/// A pinned immutable source revision for directly-sealed versions.
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// Creates a unique scratch directory for the test labeled `label`.
pub(crate) fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0u128, |elapsed| elapsed.as_nanos());
    let root = std::env::temp_dir().join(format!(
        "codex-workflow-durable-{}-{label}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create the scratch root");
    root
}

/// Best-effort removal of a scratch directory.
pub(crate) fn cleanup(root: &Path) {
    let _ = std::fs::remove_dir_all(root);
}

/// A single-step IR with no declared capabilities: runs and settles
/// without any adapter (the same shape the WO-011 plane tests use).
pub(crate) fn step_ir() -> WorkflowIr {
    let entry = IrNodeId::parse("step-001").expect("node id");
    let mut nodes = std::collections::BTreeMap::new();
    nodes.insert(
        entry.clone(),
        WorkflowIrNode::Step(StepNode {
            capabilities: Vec::new(),
            roles: Vec::new(),
            description: Some("prepare the report".to_string()),
            next: None,
        }),
    );
    WorkflowIr {
        ir_format: IR_FORMAT_VERSION,
        entry,
        nodes,
        conditions: std::collections::BTreeMap::new(),
    }
}

/// A minimal valid workflow definition over [`step_ir`].
pub(crate) fn definition(id: &str) -> WorkflowDefinition {
    WorkflowDefinition {
        id: WorkflowDefinitionId::parse(id).expect("definition id"),
        description: Some("Draft and publish release notes".to_string()),
        ir: step_ir(),
        roles: std::collections::BTreeMap::new(),
        triggers: Vec::new(),
        dependencies: codex_workflow_contracts::WorkflowDependencies::default(),
    }
}

/// Seals a definition directly through the frozen contract primitive
/// (the same `WorkflowVersion::seal` the publication pipeline ends
/// with).
pub(crate) fn sealed_version(id: &str, version: (u64, u64, u64)) -> WorkflowVersion {
    WorkflowVersion::seal(
        definition(id),
        WorkflowRepositoryId::parse("https://github.com/acme/release-bot").expect("repository id"),
        ImmutableSourceRevision::pin(
            RevisionSha::parse(COMMIT).expect("revision sha"),
            DevelopmentRef::branch("main").ok(),
        ),
        SemanticVersion::new(version.0, version.1, version.2),
        DependencyLock::default(),
        None,
    )
    .expect("version seals")
}

/// A fresh instance record for `workflow` at `version`, in `status`.
pub(crate) fn instance(
    workflow: &WorkflowDefinitionId,
    version: &WorkflowVersionId,
    status: WorkflowInstanceStatus,
) -> WorkflowInstance {
    WorkflowInstance::new(
        WorkflowInstanceId::generate(),
        workflow.clone(),
        version.clone(),
        status,
    )
}

/// A minimal installed configuration for `version`, mirroring the
/// shape the WO-011 plane records at install time.
pub(crate) fn installed(version: &WorkflowVersion) -> InstalledConfiguration {
    InstalledConfiguration {
        workflow: version.definition.id.clone(),
        version_id: version.version_id.clone(),
        reference: PublishedVersionRef::of(version),
        policy: codex_execution_contracts::BindingPolicy::default(),
        resources: Vec::new(),
        dependencies: Vec::new(),
        trigger_bindings: Vec::new(),
        schedules: Vec::new(),
        authorizations: Vec::new(),
        max_walk_steps: 1_000,
        installed_at_unix_ms: 1_000,
    }
}

/// A definition/workflow identity the tests share.
pub(crate) fn workflow_id(id: &str) -> WorkflowDefinitionId {
    WorkflowDefinitionId::parse(id).expect("definition id")
}

/// A test resource type id (credential-free by construction).
pub(crate) fn resource_type(id: &str) -> ResourceTypeId {
    ResourceTypeId::parse(id).expect("resource type")
}

/// One rebind audit record for `workflow` (fixture shape).
pub(crate) fn rebind(workflow: &str, to: &str) -> RebindRecord {
    RebindRecord {
        workflow: workflow_id(workflow),
        resource_type: resource_type("browser_profile"),
        from: None,
        to: to.to_string(),
        at_unix_ms: 100,
    }
}
