//! Installation, reconfiguration, and resource rebinding.
//!
//! Installation is the boundary between discovery and execution: an
//! installed workflow pins an immutable published version through the
//! WO-009 forge install semantics and records an explicit configuration
//! — resource bindings, dependency bindings, trigger bindings, and
//! schedules. The configuration is durable host state crossing the
//! [`crate::InstallationStore`] seam; the semantic source never changes
//! here. Every later binding change is explicit, authorized, and
//! audited.

use std::collections::BTreeSet;

use codex_execution_contracts::BindingPolicy;
use codex_execution_contracts::ResourceBinding;
use codex_workflow_app::walk::DEFAULT_WALK_BUDGET;
use codex_workflow_contracts::DependencyKey;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_forge::PublishedVersionRef;

use crate::DependencyBinding;
use crate::InstalledConfiguration;
use crate::ResourceAuthorizationRequest;
use crate::ScheduleRegistration;
use crate::ScheduleSpec;
use crate::TriggerBinding;
use crate::TriggerDiagnostic;
use crate::TriggerDiagnosticCode;
use crate::WorkflowTriggerError;
use crate::WorkflowTriggerPlane;

/// A request to install (or reconfigure) an immutable workflow version
/// with explicit bindings.
///
/// Every field is explicit by design: silent defaults for resources or
/// dependencies would violate the frozen no-silent-upgrade invariant.
/// [`InstallRequest::new`] fills only the policy/walk defaults.
#[derive(Clone, Debug)]
pub struct InstallRequest {
    /// The sealed, immutable workflow version to install.
    pub version: WorkflowVersion,
    /// Explicit resource and account bindings (opaque, credential-free).
    pub resources: Vec<ResourceBinding>,
    /// Explicit dependency implementation bindings, matching the
    /// version's dependency lock.
    pub dependencies: Vec<DependencyBinding>,
    /// The trigger classes and routing endpoints that may start the
    /// workflow.
    pub trigger_bindings: Vec<TriggerBinding>,
    /// The schedules to register.
    pub schedules: Vec<ScheduleSpec>,
    /// The binding policy applied when instances start.
    pub policy: BindingPolicy,
    /// The walk budget for started instances.
    pub max_walk_steps: u64,
}

impl InstallRequest {
    /// A minimal request: no bindings, no schedules, the conservative
    /// default policy, and the default walk budget.
    ///
    /// Versions with declared requirements or locks will refuse this
    /// minimal shape with diagnostics — configuration must be explicit.
    pub fn new(version: WorkflowVersion) -> Self {
        Self {
            version,
            resources: Vec::new(),
            dependencies: Vec::new(),
            trigger_bindings: Vec::new(),
            schedules: Vec::new(),
            policy: BindingPolicy::default(),
            max_walk_steps: DEFAULT_WALK_BUDGET,
        }
    }
}

impl WorkflowTriggerPlane {
    /// Installs a published workflow version with explicit bindings.
    ///
    /// Steps:
    ///
    /// 1. the version must be sealed and its published reference must
    ///    verify (tampered records never install);
    /// 2. the explicit configuration is validated (resources covered and
    ///    authorized, dependencies locked, trigger bindings declared,
    ///    schedules well-formed) — a failure here records nothing;
    /// 3. the forge install registry records the pin — installing over
    ///    an existing installation is rejected, so changes flow through
    ///    the explicit, reviewable update path;
    /// 4. the full version record is published into the control-plane
    ///    version store the lifecycle selects from, and the validated
    ///    configuration is stored.
    pub fn install(
        &mut self,
        request: InstallRequest,
    ) -> Result<InstalledConfiguration, WorkflowTriggerError> {
        let workflow = request.version.definition.id.clone();
        let record = request.version.clone();
        request.version.verify_integrity().map_err(|error| {
            WorkflowTriggerError::install_diagnostics(
                workflow.clone(),
                vec![TriggerDiagnostic::new(
                    TriggerDiagnosticCode::VersionIntegrity,
                    format!("the version failed integrity verification: {error}"),
                )],
            )
        })?;
        let reference = PublishedVersionRef::of(&request.version);
        reference.verify()?;
        // Validate the explicit configuration before recording anything:
        // failed installs leave no residue in any seam.
        let configuration = self.validate_configuration(workflow, request)?;
        // Forge install semantics: no silent over-install; upgrades go
        // through propose_update/decide_update plus reconfigure.
        self.installs.install(&reference)?;
        // Mirror the sealed record into the shared control-plane store.
        self.versions.publish(record)?;
        self.installations.save(configuration.clone())?;
        Ok(configuration)
    }

    /// Reconfigures an installation for the version the forge registry
    /// currently installs.
    ///
    /// This is the completion step of an explicit update: after
    /// `propose_update`/`decide_update` moved the forge installation,
    /// the configuration is re-validated and re-bound to exactly that
    /// version. A request naming any other version is refused with
    /// [`WorkflowTriggerError::VersionDrift`] — the silent-upgrade
    /// guard. Already-running instances keep the version they pinned.
    pub fn reconfigure(
        &mut self,
        workflow: &codex_workflow_contracts::WorkflowDefinitionId,
        request: InstallRequest,
    ) -> Result<InstalledConfiguration, WorkflowTriggerError> {
        let record = request.version.clone();
        request.version.verify_integrity().map_err(|error| {
            WorkflowTriggerError::install_diagnostics(
                workflow.to_string(),
                vec![TriggerDiagnostic::new(
                    TriggerDiagnosticCode::VersionIntegrity,
                    format!("the version failed integrity verification: {error}"),
                )],
            )
        })?;
        let reference = PublishedVersionRef::of(&request.version);
        reference.verify()?;
        let installed = self.installs.installed(workflow).ok_or_else(|| {
            WorkflowTriggerError::NotInstalled {
                workflow: workflow.to_string(),
            }
        })?;
        if installed.installed.version_id != request.version.version_id {
            return Err(WorkflowTriggerError::VersionDrift {
                workflow: workflow.to_string(),
                requested: request.version.version_id.to_string(),
                installed: installed.installed.version_id.to_string(),
            });
        }
        // Validate the explicit configuration before recording anything.
        let configuration = self.validate_configuration(workflow.clone(), request)?;
        self.versions.publish(record)?;
        self.installations.save(configuration.clone())?;
        Ok(configuration)
    }

    /// Rebinds one resource or account type to a new concrete instance.
    ///
    /// The replacement must target a resource type the version declares
    /// or one the configuration already binds (adapter-side resource
    /// types enter the configuration at install time), and must be
    /// authorized by the authorization plane. The rebind swaps the
    /// configuration binding, records an audited [`crate::RebindRecord`],
    /// and never touches the immutable version record — the semantic
    /// source, its digests, and running instances' pinned versions are
    /// unchanged. Future fires bind the new resource.
    pub fn rebind_resource(
        &mut self,
        workflow: &codex_workflow_contracts::WorkflowDefinitionId,
        replacement: ResourceBinding,
    ) -> Result<InstalledConfiguration, WorkflowTriggerError> {
        let mut configuration = self.installations.load(workflow)?.ok_or_else(|| {
            WorkflowTriggerError::NotInstalled {
                workflow: workflow.to_string(),
            }
        })?;
        let version = self
            .versions
            .load(&configuration.version_id)?
            .ok_or_else(|| WorkflowTriggerError::VersionUnavailable {
                version: configuration.version_id.to_string(),
            })?;
        version.verify_integrity()?;

        let mut diagnostics = Vec::new();
        let declared = version
            .definition
            .dependencies
            .resources
            .iter()
            .any(|requirement| requirement.resource_type == replacement.resource_type)
            || configuration
                .resources
                .iter()
                .any(|binding| binding.resource_type == replacement.resource_type);
        if !declared {
            diagnostics.push(TriggerDiagnostic::new(
                TriggerDiagnosticCode::ResourceNotDeclared,
                format!(
                    "rebind targets type `{}`, which the version does not declare and the configuration does not bind",
                    replacement.resource_type.as_ref()
                ),
            ));
        }
        let authorization = self.authorizer.authorize(&ResourceAuthorizationRequest {
            workflow: workflow.clone(),
            binding: replacement.clone(),
        })?;
        if !authorization.authorized {
            diagnostics.push(TriggerDiagnostic::new(
                TriggerDiagnosticCode::ResourceNotAuthorized,
                format!(
                    "replacement resource `{}` ({}) is not authorized{}",
                    replacement.resource.as_ref(),
                    replacement.resource_type.as_ref(),
                    authorization
                        .reason
                        .as_ref()
                        .map(|reason| format!(": {reason}"))
                        .unwrap_or_default()
                ),
            ));
        }
        if !diagnostics.is_empty() {
            return Err(WorkflowTriggerError::install_diagnostics(
                workflow.to_string(),
                diagnostics,
            ));
        }

        let position = configuration
            .resources
            .iter()
            .position(|binding| binding.resource_type == replacement.resource_type);
        let from =
            position.map(|index| configuration.resources[index].resource.as_ref().to_string());
        match position {
            Some(index) => configuration.resources[index] = replacement.clone(),
            None => configuration.resources.push(replacement.clone()),
        }
        configuration.authorizations.push(authorization);
        let audit = crate::RebindRecord {
            workflow: workflow.clone(),
            resource_type: replacement.resource_type.clone(),
            from,
            to: replacement.resource.as_ref().to_string(),
            at_unix_ms: self.clock.now_unix_ms(),
        };
        self.installations.append_audit(audit)?;
        self.installations.save(configuration.clone())?;
        Ok(configuration)
    }

    /// Validates an explicit configuration against the sealed version
    /// and builds the installed record.
    ///
    /// Checks: resource coverage (every declared requirement bound;
    /// adapter-side types may be bound additionally), authorization of
    /// every resource binding, dependency-lock coverage and integrity
    /// (every locked dependency bound with a matching digest; no extra
    /// bindings), trigger declarations, and schedule well-formedness.
    fn validate_configuration(
        &mut self,
        workflow: codex_workflow_contracts::WorkflowDefinitionId,
        request: InstallRequest,
    ) -> Result<InstalledConfiguration, WorkflowTriggerError> {
        let version = request.version;
        let mut diagnostics = Vec::new();

        // Resource coverage: every requirement the version *declares*
        // must be bound. Adapter-side resource types (discovered by the
        // execution plane at binding time, not declared by the taught
        // definition) may be bound explicitly as well — every binding is
        // authorized either way.
        for requirement in &version.definition.dependencies.resources {
            if !request
                .resources
                .iter()
                .any(|binding| binding.resource_type == requirement.resource_type)
            {
                diagnostics.push(TriggerDiagnostic::new(
                    TriggerDiagnosticCode::ResourceMissing,
                    format!(
                        "declared resource requirement `{}` has no binding",
                        requirement.resource_type.as_ref()
                    ),
                ));
            }
        }

        // Authorization of every resource binding.
        let mut authorizations = Vec::new();
        for binding in &request.resources {
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
            authorizations.push(decision);
        }

        // Dependency bindings: coverage and lock integrity.
        for (key, entry) in &version.dependency_lock.entries {
            let compact = key.as_key();
            match request
                .dependencies
                .iter()
                .find(|binding| binding.key == compact)
            {
                None => diagnostics.push(TriggerDiagnostic::new(
                    TriggerDiagnosticCode::DependencyUnbound,
                    format!("locked dependency `{compact}` has no binding"),
                )),
                Some(binding) => {
                    if binding.integrity != entry.resolved.integrity_digest() {
                        diagnostics.push(TriggerDiagnostic::new(
                            TriggerDiagnosticCode::DependencyDrift,
                            format!(
                                "dependency binding `{}` for `{compact}` does not match the version's immutable dependency lock",
                                binding.binding
                            ),
                        ));
                    }
                }
            }
        }
        let locked_keys: BTreeSet<String> = version
            .dependency_lock
            .entries
            .keys()
            .map(DependencyKey::as_key)
            .collect();
        for binding in &request.dependencies {
            if !locked_keys.contains(&binding.key) {
                diagnostics.push(TriggerDiagnostic::new(
                    TriggerDiagnosticCode::DependencyDrift,
                    format!(
                        "dependency binding `{}` targets `{}`, which the version's dependency lock does not contain",
                        binding.binding, binding.key
                    ),
                ));
            }
        }

        // Trigger declarations, when the version normalizes them.
        if !version.definition.triggers.is_empty() {
            for binding in &request.trigger_bindings {
                if !version
                    .definition
                    .triggers
                    .iter()
                    .any(|declared| declared.trigger == binding.trigger)
                {
                    diagnostics.push(TriggerDiagnostic::new(
                        TriggerDiagnosticCode::TriggerNotDeclared,
                        format!(
                            "trigger binding {:?} is not declared by the version",
                            binding.trigger
                        ),
                    ));
                }
            }
        }

        // Schedules.
        for spec in &request.schedules {
            spec.validate()?;
        }

        if !diagnostics.is_empty() {
            return Err(WorkflowTriggerError::install_diagnostics(
                workflow,
                diagnostics,
            ));
        }

        let now = self.clock.now_unix_ms();
        let schedules = request
            .schedules
            .into_iter()
            .map(|spec| {
                self.schedule_counter += 1;
                ScheduleRegistration {
                    schedule_id: format!("schedule-{}", self.schedule_counter),
                    spec,
                    anchor_unix_ms: now,
                    cursor_unix_ms: now,
                }
            })
            .collect();
        Ok(InstalledConfiguration {
            workflow,
            version_id: version.version_id.clone(),
            reference: PublishedVersionRef::of(&version),
            policy: request.policy,
            resources: request.resources,
            dependencies: request.dependencies,
            trigger_bindings: request.trigger_bindings,
            schedules,
            authorizations,
            max_walk_steps: request.max_walk_steps,
            installed_at_unix_ms: now,
        })
    }
}
