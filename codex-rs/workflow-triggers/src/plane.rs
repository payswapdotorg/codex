//! The workflow trigger plane: ingest, gate, fire, resume, and poll.
//!
//! [`WorkflowTriggerPlane`] composes the frozen planes behind
//! control-plane seams. It owns no durable state of its own: every
//! durable decision crosses a port seam (see the re-exported traits) or the WO-010
//! [`WorkflowLifecycle`] it drives by reference. Its operations are the
//! application-level run control surface of roadmap M10.
//!
//! Fire path (one event, one workflow):
//!
//! ```text
//! ledger.accept     -- idempotency: first sight allocates an event id,
//!                      repeats are recorded no-ops
//! evaluate_gate     -- installation, forge consistency, version
//!                      integrity, trigger bindings and declarations,
//!                      resource coverage, dependency-lock match,
//!                      fresh resource authorization
//! awaiting          -- resume correlation: a paused instance waiting
//!                      for this trigger class resumes instead of
//!                      starting a new instance
//! lifecycle         -- select_version -> instantiate (validate ->
//!                      approve -> bind) -> run; nothing dispatches
//!                      unless every gate passed
//! ledger.settle     -- the outcome is recorded, including the await
//!                      registration of a paused run
//! ```

use codex_workflow_app::InstantiateRequest;
use codex_workflow_app::RunOutcome;
use codex_workflow_app::RunTerminal;
use codex_workflow_app::WalkConfig;
use codex_workflow_app::WorkflowAppError;
use codex_workflow_app::WorkflowLifecycle;
use codex_workflow_app::port::WorkflowVersionStore;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::TriggerSource;
use codex_workflow_contracts::WaitFor;
use codex_workflow_contracts::WaitNode;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_forge::InstallRegistry;

use crate::FireOutcome;
use crate::IncomingTrigger;
use crate::InstalledConfiguration;
use crate::InstanceSettlement;
use crate::ResourceAuthorizationRequest;
use crate::ResumeDirective;
use crate::TriggerAcceptance;
use crate::TriggerDiagnostic;
use crate::TriggerDiagnosticCode;
use crate::WorkflowTriggerError;
use crate::due_occurrences;

/// The seams a trigger plane is constructed from.
///
/// Every durable or host-owned concern crosses one of these boxes; the
/// plane composes the WO-010 [`WorkflowLifecycle`] by reference at
/// operation time. The `versions` store **must be backed by the same
/// durable store the lifecycle selects from** — the plane publishes
/// installed versions into it and re-verifies integrity at fire time.
pub struct TriggerDeps {
    /// The durable trigger idempotency and audit ledger.
    pub ledger: Box<dyn crate::TriggerLedger>,
    /// The discovery transport (forge search, releases).
    pub catalog: Box<dyn crate::PackageCatalog>,
    /// The resource and account authorization plane.
    pub authorizer: Box<dyn crate::ResourceAuthorizer>,
    /// The resume seam into the durable instance control plane.
    pub control: Box<dyn crate::InstanceControl>,
    /// Durable installed configurations and rebind audits.
    pub installations: Box<dyn crate::InstallationStore>,
    /// The scheduler clock.
    pub clock: Box<dyn crate::ScheduleClock>,
    /// The control-plane workflow version store (shared with the
    /// lifecycle).
    pub versions: Box<dyn WorkflowVersionStore>,
    /// The forge install registry (in-process record collection; the
    /// host mirrors it durably through the control plane).
    pub installs: InstallRegistry,
}

/// The outcome of one trigger fire, as reported to the caller.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TriggerReport {
    /// The control-plane event identity allocated for this fire.
    pub event_id: String,
    /// What the fire did (the same record the ledger settled).
    pub outcome: FireOutcome,
}

/// The static eligibility verdict before any instance is created.
enum GateEvaluation {
    /// Every static check passed; the configuration and sealed version
    /// needed to start.
    Eligible(Box<EligibleGate>),
    /// The fire is rejected with structured diagnostics; no instance
    /// record is created.
    Rejected { diagnostics: Vec<TriggerDiagnostic> },
}

/// The payload of an eligible gate evaluation.
struct EligibleGate {
    configuration: InstalledConfiguration,
    version: WorkflowVersion,
}

/// The trigger/scheduling/installation plane over the WO-010 workflow
/// application ports.
pub struct WorkflowTriggerPlane {
    pub(crate) ledger: Box<dyn crate::TriggerLedger>,
    pub(crate) catalog: Box<dyn crate::PackageCatalog>,
    pub(crate) authorizer: Box<dyn crate::ResourceAuthorizer>,
    pub(crate) control: Box<dyn crate::InstanceControl>,
    pub(crate) installations: Box<dyn crate::InstallationStore>,
    pub(crate) clock: Box<dyn crate::ScheduleClock>,
    pub(crate) versions: Box<dyn WorkflowVersionStore>,
    pub(crate) installs: InstallRegistry,
    pub(crate) schedule_counter: u64,
}

impl WorkflowTriggerPlane {
    /// Creates the plane from its seams.
    ///
    /// Nothing is probed, installed, or fired at construction: the plane
    /// is inert until the application drives it, and with no installed
    /// configuration every operation is a recorded rejection.
    pub fn new(deps: TriggerDeps) -> Self {
        Self {
            ledger: deps.ledger,
            catalog: deps.catalog,
            authorizer: deps.authorizer,
            control: deps.control,
            installations: deps.installations,
            clock: deps.clock,
            versions: deps.versions,
            installs: deps.installs,
            schedule_counter: 0,
        }
    }

    /// The forge install registry backing this plane (explicit, reviewable
    /// update plans flow through it).
    pub fn install_registry(&self) -> &InstallRegistry {
        &self.installs
    }

    /// Mutable access to the forge install registry, for driving explicit
    /// update plans and decisions.
    pub fn install_registry_mut(&mut self) -> &mut InstallRegistry {
        &mut self.installs
    }

    /// Ingests one trigger event.
    ///
    /// When the envelope targets a workflow explicitly, exactly that
    /// workflow is fired; otherwise the plane routes by the installed
    /// trigger bindings matching the event class and routing endpoint.
    /// Each target crosses the full fire path; an event matching no
    /// binding is inert (no reports, no records).
    pub async fn ingest(
        &mut self,
        lifecycle: &mut WorkflowLifecycle,
        envelope: IncomingTrigger,
    ) -> Result<Vec<TriggerReport>, WorkflowTriggerError> {
        let targets: Vec<WorkflowDefinitionId> = match &envelope.target {
            Some(workflow) => vec![workflow.clone()],
            None => self.route(envelope.trigger, envelope.routing.as_deref())?,
        };
        let mut reports = Vec::new();
        for workflow in targets {
            let report = self.fire_one(lifecycle, &workflow, &envelope).await?;
            reports.push(report);
        }
        Ok(reports)
    }

    /// Polls every registered schedule and fires the due occurrences.
    ///
    /// The scheduler owns no authority of its own: each due occurrence
    /// becomes a trigger envelope with a deterministic idempotency key
    /// and crosses the exact same fire path as every other event, so a
    /// re-poll after a crash is a stream of recorded no-ops. The
    /// consumption cursor only moves forward, and occurrences skipped
    /// by the catch-up cap are consumed rather than replayed.
    pub async fn poll_schedules(
        &mut self,
        lifecycle: &mut WorkflowLifecycle,
    ) -> Result<Vec<TriggerReport>, WorkflowTriggerError> {
        let now = self.clock.now_unix_ms();
        let mut reports = Vec::new();
        for mut configuration in self.installations.list()? {
            let mut changed = false;
            for schedule in &mut configuration.schedules {
                let due = due_occurrences(
                    &schedule.spec,
                    schedule.anchor_unix_ms,
                    schedule.cursor_unix_ms,
                    now,
                );
                for occurrence in due {
                    let envelope = IncomingTrigger::scheduled(
                        &configuration.workflow,
                        &schedule.schedule_id,
                        occurrence,
                    )?;
                    let report = self
                        .fire_one(lifecycle, &configuration.workflow, &envelope)
                        .await?;
                    reports.push(report);
                }
                if now > schedule.cursor_unix_ms {
                    schedule.cursor_unix_ms = now;
                    changed = true;
                }
            }
            if changed {
                self.installations.save(configuration)?;
            }
        }
        Ok(reports)
    }

    /// The workflows whose installed trigger bindings accept `class` on
    /// `endpoint`, ordered by workflow identity.
    fn route(
        &self,
        class: TriggerClass,
        endpoint: Option<&str>,
    ) -> Result<Vec<WorkflowDefinitionId>, WorkflowTriggerError> {
        Ok(self
            .installations
            .list()?
            .into_iter()
            .filter(|configuration| {
                configuration
                    .trigger_bindings
                    .iter()
                    .any(|binding| binding.matches(class, endpoint))
            })
            .map(|configuration| configuration.workflow)
            .collect())
    }

    /// The full fire path for one event on one workflow.
    async fn fire_one(
        &mut self,
        lifecycle: &mut WorkflowLifecycle,
        workflow: &WorkflowDefinitionId,
        envelope: &IncomingTrigger,
    ) -> Result<TriggerReport, WorkflowTriggerError> {
        // 1. Idempotent acceptance through the control-plane ledger.
        let now = self.clock.now_unix_ms();
        let acceptance = self.ledger.accept(workflow, envelope, now)?;
        let event_id = match acceptance {
            TriggerAcceptance::Accepted { event_id } => event_id,
            TriggerAcceptance::Duplicate { event_id } => {
                // The recorded no-op: the original acceptance already
                // drove the only instance transition this event may
                // cause.
                self.ledger.settle(&event_id, FireOutcome::Duplicate)?;
                return Ok(TriggerReport {
                    event_id,
                    outcome: FireOutcome::Duplicate,
                });
            }
        };

        // 2. Static eligibility gate (before any instance exists).
        let evaluation =
            self.evaluate_gate(workflow, envelope.trigger, envelope.routing.as_deref())?;
        let outcome = match evaluation {
            GateEvaluation::Eligible(gate) => {
                self.start_or_resume(
                    lifecycle,
                    workflow,
                    &event_id,
                    envelope,
                    gate.configuration,
                    gate.version,
                )
                .await?
            }
            GateEvaluation::Rejected { diagnostics } => FireOutcome::NotEligible { diagnostics },
        };

        // 3. The outcome is durably settled, including the await
        //    registration of a paused run.
        self.ledger.settle(&event_id, outcome.clone())?;
        Ok(TriggerReport { event_id, outcome })
    }

    /// Resumes correlated awaits, or starts one new instance through the
    /// WO-010 lifecycle.
    async fn start_or_resume(
        &mut self,
        lifecycle: &mut WorkflowLifecycle,
        workflow: &WorkflowDefinitionId,
        event_id: &str,
        envelope: &IncomingTrigger,
        configuration: InstalledConfiguration,
        version: WorkflowVersion,
    ) -> Result<FireOutcome, WorkflowTriggerError> {
        // Resume correlation: an event satisfying an outstanding wait
        // resumes the paused instance instead of spawning more work.
        let awaiting = self.ledger.awaiting(workflow, envelope.trigger)?;
        if !awaiting.is_empty() {
            let mut instances = Vec::new();
            for record in awaiting {
                let directive = ResumeDirective {
                    instance: record.instance,
                    node: record.node,
                    trigger: TriggerSource {
                        trigger: envelope.trigger,
                        event_id: Some(event_id.to_string()),
                    },
                };
                self.control.resume(&directive)?;
                instances.push(record.instance);
            }
            return Ok(FireOutcome::Resumed { instances });
        }

        // Start: the lifecycle enforces its own gates (integrity,
        // readiness, approvals, binding) before anything dispatches.
        if let Err(error) = lifecycle.select_version(&version.version_id) {
            return Ok(FireOutcome::NotEligible {
                diagnostics: vec![selection_diagnostic(error)],
            });
        }
        let request = InstantiateRequest {
            trigger: Some(TriggerSource {
                trigger: envelope.trigger,
                event_id: Some(event_id.to_string()),
            }),
            policy: configuration.policy.clone(),
            resources: configuration.resources.clone(),
            walk: WalkConfig {
                max_steps: configuration.max_walk_steps,
            },
        };
        let instance = match lifecycle.instantiate(request).await {
            Ok(instance) => instance,
            Err(error) => {
                return Ok(FireOutcome::InstantiationFailed {
                    diagnostics: instantiation_diagnostics(error),
                });
            }
        };
        let instance_id = instance.instance_id;
        match lifecycle.run().await {
            Ok(outcome) => Ok(FireOutcome::Started {
                instance: instance_id,
                status: settlement_from_outcome(&version, &outcome),
            }),
            Err(error) => Ok(FireOutcome::Started {
                instance: instance_id,
                status: InstanceSettlement::Failed {
                    reason: error.to_string(),
                },
            }),
        }
    }

    /// The static eligibility gate: everything checked before any
    /// instance record exists.
    ///
    /// Capability, policy, and binding-plan readiness is enforced one
    /// step later, inside instantiation (which settles a `Failed`
    /// instance with structured diagnostics); the static gate here
    /// guarantees that gate is only reached for properly configured
    /// fires.
    fn evaluate_gate(
        &mut self,
        workflow: &WorkflowDefinitionId,
        class: TriggerClass,
        endpoint: Option<&str>,
    ) -> Result<GateEvaluation, WorkflowTriggerError> {
        let mut diagnostics = Vec::new();
        let Some(configuration) = self.installations.load(workflow)? else {
            return Ok(GateEvaluation::Rejected {
                diagnostics: vec![TriggerDiagnostic::new(
                    TriggerDiagnosticCode::NotInstalled,
                    format!("workflow `{workflow}` has no installed configuration"),
                )],
            });
        };

        // Forge installation consistency: the silent-upgrade guard.
        match self.installs.installed(workflow) {
            Some(record) => {
                if record.installed.version_id != configuration.version_id {
                    diagnostics.push(TriggerDiagnostic::new(
                        TriggerDiagnosticCode::VersionDrift,
                        format!(
                            "the configuration pins `{}` but the forge registry installs `{}`; \
                             reconfigure the installation explicitly",
                            configuration.version_id, record.installed.version_id
                        ),
                    ));
                }
            }
            None => diagnostics.push(TriggerDiagnostic::new(
                TriggerDiagnosticCode::NotInstalled,
                format!("workflow `{workflow}` has no forge installation record"),
            )),
        }

        // The sealed version record: present and integrity-verified.
        let version = match self.versions.load(&configuration.version_id)? {
            Some(record) => match record.verify_integrity() {
                Ok(()) => Some(record),
                Err(error) => {
                    diagnostics.push(TriggerDiagnostic::new(
                        TriggerDiagnosticCode::VersionIntegrity,
                        format!(
                            "version `{}` failed integrity verification: {error}",
                            configuration.version_id
                        ),
                    ));
                    None
                }
            },
            None => {
                diagnostics.push(TriggerDiagnostic::new(
                    TriggerDiagnosticCode::VersionUnavailable,
                    format!(
                        "version `{}` is not present in the version store",
                        configuration.version_id
                    ),
                ));
                None
            }
        };

        // Trigger routing: an unbound class/endpoint never starts the
        // workflow.
        if configuration
            .trigger_binding_matching(class, endpoint)
            .is_none()
        {
            diagnostics.push(TriggerDiagnostic::new(
                TriggerDiagnosticCode::TriggerNotBound,
                format!(
                    "no installed trigger binding accepts {class:?} events on endpoint `{}`",
                    endpoint.unwrap_or("(direct)")
                ),
            ));
        }

        if let Some(record) = &version {
            // Trigger declarations, when the version normalizes them.
            if !record.definition.triggers.is_empty()
                && !record
                    .definition
                    .triggers
                    .iter()
                    .any(|declared| declared.trigger == class)
            {
                diagnostics.push(TriggerDiagnostic::new(
                    TriggerDiagnosticCode::TriggerNotDeclared,
                    format!("the version declares its triggers and does not declare {class:?}"),
                ));
            }
            // Resource coverage.
            for requirement in &record.definition.dependencies.resources {
                if configuration
                    .resource_binding(&requirement.resource_type)
                    .is_none()
                {
                    diagnostics.push(TriggerDiagnostic::new(
                        TriggerDiagnosticCode::ResourceMissing,
                        format!(
                            "declared resource requirement `{}` has no configured binding",
                            requirement.resource_type.as_ref()
                        ),
                    ));
                }
            }
            // Dependency bindings: coverage and lock integrity (the
            // silent-upgrade guard for dependencies).
            for (key, entry) in &record.dependency_lock.entries {
                let compact = key.as_key();
                match configuration
                    .dependencies
                    .iter()
                    .find(|binding| binding.key == compact)
                {
                    None => diagnostics.push(TriggerDiagnostic::new(
                        TriggerDiagnosticCode::DependencyUnbound,
                        format!("locked dependency `{compact}` has no configured binding"),
                    )),
                    Some(binding) => {
                        if binding.integrity != entry.resolved.integrity_digest() {
                            diagnostics.push(TriggerDiagnostic::new(
                                TriggerDiagnosticCode::DependencyDrift,
                                format!(
                                    "dependency binding `{}` for `{compact}` does not match the \
                                     version's immutable dependency lock",
                                    binding.binding
                                ),
                            ));
                        }
                    }
                }
            }
            let locked_keys: Vec<String> = record
                .dependency_lock
                .entries
                .keys()
                .map(codex_workflow_contracts::DependencyKey::as_key)
                .collect();
            for binding in &configuration.dependencies {
                if !locked_keys.contains(&binding.key) {
                    diagnostics.push(TriggerDiagnostic::new(
                        TriggerDiagnosticCode::DependencyDrift,
                        format!(
                            "dependency binding `{}` targets `{}`, which the version's \
                             dependency lock does not contain",
                            binding.binding, binding.key
                        ),
                    ));
                }
            }
        }

        // Fresh resource authorization: availability was never
        // authorization.
        for binding in &configuration.resources {
            let decision = self.authorizer.authorize(&ResourceAuthorizationRequest {
                workflow: workflow.clone(),
                binding: binding.clone(),
            })?;
            if !decision.authorized {
                diagnostics.push(TriggerDiagnostic::new(
                    TriggerDiagnosticCode::ResourceNotAuthorized,
                    format!(
                        "resource `{}` ({}) is not authorized{}",
                        binding.resource.as_ref(),
                        binding.resource_type.as_ref(),
                        decision
                            .reason
                            .as_ref()
                            .map(|reason| format!(": {reason}"))
                            .unwrap_or_default()
                    ),
                ));
            }
        }

        if !diagnostics.is_empty() {
            return Ok(GateEvaluation::Rejected { diagnostics });
        }
        match version {
            Some(version) => Ok(GateEvaluation::Eligible(Box::new(EligibleGate {
                configuration,
                version,
            }))),
            None => Ok(GateEvaluation::Rejected {
                diagnostics: vec![TriggerDiagnostic::new(
                    TriggerDiagnosticCode::VersionUnavailable,
                    "the version record became unavailable during gating",
                )],
            }),
        }
    }
}

/// Maps a run outcome to its durable settlement, including the
/// trigger-await correlation of a pause.
fn settlement_from_outcome(version: &WorkflowVersion, outcome: &RunOutcome) -> InstanceSettlement {
    match &outcome.terminal {
        RunTerminal::Completed => InstanceSettlement::Completed,
        RunTerminal::Paused { node, .. } => InstanceSettlement::Paused {
            node: node.clone(),
            awaiting: awaited_trigger(&version.definition.ir, node),
        },
        RunTerminal::Failed { reason } => InstanceSettlement::Failed {
            reason: reason.clone(),
        },
    }
}

/// The trigger class a wait node awaits, when it waits for a trigger.
fn awaited_trigger(ir: &WorkflowIr, node: &IrNodeId) -> Option<TriggerClass> {
    match ir.nodes.get(node) {
        Some(WorkflowIrNode::Wait(WaitNode {
            wait_for: WaitFor::Trigger(class),
            ..
        })) => Some(*class),
        _ => None,
    }
}

/// Maps a late version-selection failure (after acceptance and gating)
/// to a diagnostic.
fn selection_diagnostic(error: WorkflowAppError) -> TriggerDiagnostic {
    let code = match &error {
        WorkflowAppError::VersionIntegrity { .. } => TriggerDiagnosticCode::VersionIntegrity,
        _ => TriggerDiagnosticCode::VersionUnavailable,
    };
    TriggerDiagnostic::new(
        code,
        format!("version selection failed after acceptance: {error}"),
    )
}

/// Maps an instantiation failure to its diagnostics: readiness and
/// policy refusals keep their structured shape; everything else is
/// bounded detail.
fn instantiation_diagnostics(error: WorkflowAppError) -> Vec<TriggerDiagnostic> {
    match error {
        WorkflowAppError::BindingPlanFailed { code, message } => {
            vec![TriggerDiagnostic::new(
                TriggerDiagnosticCode::ReadinessDenied,
                format!("binding plan failed: {code:?}: {message}"),
            )]
        }
        WorkflowAppError::ApprovalDenied { binding, reason } => {
            vec![TriggerDiagnostic::new(
                TriggerDiagnosticCode::ReadinessDenied,
                format!("approval for binding `{binding}` was denied: {reason}"),
            )]
        }
        other => vec![TriggerDiagnostic::new(
            TriggerDiagnosticCode::InstantiationFailed,
            format!("{other}"),
        )],
    }
}
