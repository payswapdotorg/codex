//! Evidence-driven candidate generation.
//!
//! The generator is the learning surface of this crate: it turns a
//! read-only [`EvidenceCorpus`] — execution evidence references, recorded
//! run records, observed environment classes, and the installed immutable
//! version — into [`ImprovementCandidate`]s with full provenance.
//!
//! # What the generator may never do
//!
//! - It never reads live engine state: the corpus is a closed, read-only
//!   snapshot.
//! - It never treats model output as authorization: no model, provider, or
//!   agent type appears in its API; suggestions from models are a host
//!   concern, and only the approval plane authorizes promotion.
//! - It never embeds evidence payloads: provenance cites references and
//!   run fingerprints only.
//!
//! # Deterministic rules
//!
//! Every rule maps one evidence stream to one change category and fires
//! only on that stream:
//!
//! | evidence stream | rule | proposed change |
//! |---|---|---|
//! | escalations or recovery records present | scope resolution to observed environments | [`Recovery`](ChangeKind::RecoveryPolicy) |
//! | observation references present | restate step bindings with evidence-cited purposes | [`Bindings`](ChangeKind::CapabilityBinding) |
//! | trace references present | refresh the definition description from the traces | [`Definition`](ChangeKind::DefinitionDelta) |
//! | lock entries beyond the declared dependencies | minimize the lock to the declared set | [`Dependency`](ChangeKind::DependencyChoice) |
//! | observed schedule and fully healthy runs | halve the schedule period | [`Schedule`](ChangeKind::ScheduleTuning) |
//!
//! The rules are deliberately conservative heuristics: their output is a
//! *proposal* whose worth is decided by the validation pipeline and the
//! approval plane, never by the generator itself.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use codex_eval_compat::WorkflowRunRecord;
use codex_execution_contracts::BindingPolicy;
use codex_execution_contracts::EnvironmentScope;
use codex_execution_contracts::ExecutionEnvironment;
use codex_execution_contracts::HumanFallbackPolicy;
use codex_workflow_contracts::CapabilityRequirement;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::DependencyKey;
use codex_workflow_contracts::DependencyLock;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;
use codex_workflow_contracts::WorkflowDefinition;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_contracts::WorkflowIrNode;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;
use codex_workflow_forge::PublishedVersionRef;
use codex_workflow_triggers::ScheduleSpec;

use crate::CandidateId;
use crate::CandidateProvenance;
use crate::ChangeKind;
use crate::ImprovementCandidate;
use crate::ProposedChange;
use crate::RunProvenance;
use crate::WorkflowEvolutionError;

/// The read-only evidence snapshot the generator learns from.
///
/// The corpus combines four closed inputs:
///
/// - the installed immutable version (verified on construction) whose
///   definition and lock the candidate proposes against;
/// - the recorded evaluation runs of that version, as run records;
/// - the execution evidence references those runs recorded;
/// - host-recorded observations (environment classes that served the runs,
///   the installed trigger schedule).
///
/// Nothing here is live engine state, and nothing in it is a model
/// suggestion: hosts build the corpus from their durable evidence plane.
#[derive(Clone, Debug)]
pub struct EvidenceCorpus {
    /// The immutable version the corpus describes.
    pub incumbent: PublishedVersionRef,
    /// The incumbent's definition (read-only copy).
    pub definition: WorkflowDefinition,
    /// The incumbent's dependency lock (read-only copy).
    pub dependency_lock: DependencyLock,
    /// The execution evidence references recorded by the runs.
    pub references: Vec<EvidenceReference>,
    /// The content-addressed identities of the recorded runs.
    pub runs: Vec<RunProvenance>,
    /// Total escalations across the recorded runs.
    pub escalations: usize,
    /// The environment classes observed serving the runs.
    pub observed_environments: BTreeSet<ExecutionEnvironment>,
    /// The schedule observed installed for the workflow, when any.
    pub observed_schedule: Option<ScheduleSpec>,
}

impl EvidenceCorpus {
    /// Builds a corpus from the installed version and its recorded
    /// observations.
    ///
    /// The incumbent version must verify end to end; run records must pin
    /// exactly the incumbent version (evidence about other versions does
    /// not describe this workflow installation).
    pub fn from_observations(
        incumbent: &WorkflowVersion,
        runs: &[WorkflowRunRecord],
        references: Vec<EvidenceReference>,
        observed_environments: BTreeSet<ExecutionEnvironment>,
        observed_schedule: Option<ScheduleSpec>,
    ) -> Result<Self, WorkflowEvolutionError> {
        incumbent.verify_integrity()?;
        let incumbent_ref = PublishedVersionRef::of(incumbent);
        let mut provenances = Vec::with_capacity(runs.len());
        let mut escalations = 0;
        for record in runs {
            if record.version != incumbent.version_id {
                return Err(WorkflowEvolutionError::InvalidCandidate {
                    candidate: "corpus".to_string(),
                    reason: format!(
                        "run record pins version `{}`, not the installed incumbent `{}`",
                        record.version, incumbent.version_id
                    ),
                });
            }
            escalations += record.escalations;
            provenances.push(RunProvenance::of_record(record)?);
        }
        Ok(Self {
            incumbent: incumbent_ref,
            definition: incumbent.definition.clone(),
            dependency_lock: incumbent.dependency_lock.clone(),
            references,
            runs: provenances,
            escalations,
            observed_environments,
            observed_schedule,
        })
    }

    /// The workflow the corpus describes.
    pub fn workflow(&self) -> &WorkflowDefinitionId {
        &self.incumbent.identity.workflow
    }

    /// How many references of one evidence kind the corpus carries.
    fn count_kind(&self, kind: EvidenceKind) -> usize {
        self.references
            .iter()
            .filter(|reference| reference.kind == kind)
            .count()
    }
}

/// Tuning knobs of the generator.
#[derive(Clone, Debug)]
pub struct GenerationPolicy {
    /// The namespace prefix of derived candidate ids: non-empty lowercase
    /// letters, digits, or `-`, at most 48 characters.
    pub id_prefix: String,
}

impl Default for GenerationPolicy {
    fn default() -> Self {
        Self {
            id_prefix: "evolution".to_string(),
        }
    }
}

/// The deterministic, evidence-driven candidate generator.
///
/// The generator is pure: the same corpus yields the same candidates in
/// the same order, which is what makes candidate ids content-addressed and
/// regeneration idempotent.
#[derive(Clone, Debug)]
pub struct CandidateGenerator {
    policy: GenerationPolicy,
}

impl CandidateGenerator {
    /// Creates a generator with the default policy.
    pub fn new() -> Self {
        Self {
            policy: GenerationPolicy::default(),
        }
    }

    /// Creates a generator with an explicit policy.
    ///
    /// The id prefix is validated eagerly so derived candidate ids can
    /// never fail to parse afterwards.
    pub fn with_policy(policy: GenerationPolicy) -> Result<Self, WorkflowEvolutionError> {
        let prefix = &policy.id_prefix;
        let valid = !prefix.is_empty()
            && prefix.len() <= 48
            && prefix.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
            });
        if !valid {
            return Err(WorkflowEvolutionError::InvalidCandidate {
                candidate: prefix.clone(),
                reason: "generation id prefixes must be non-empty lowercase/digits/hyphen, at most 48 characters"
                    .to_string(),
            });
        }
        Ok(Self { policy })
    }

    /// Generates every improvement candidate the corpus supports.
    ///
    /// Order is deterministic: recovery, bindings, definition, dependency,
    /// schedule (the fixed rule order of this module's table).
    pub fn generate(
        &self,
        corpus: &EvidenceCorpus,
    ) -> Result<Vec<ImprovementCandidate>, WorkflowEvolutionError> {
        let mut candidates = Vec::new();
        if let Some(candidate) = self.recovery_candidate(corpus)? {
            candidates.push(candidate);
        }
        if let Some(candidate) = self.bindings_candidate(corpus)? {
            candidates.push(candidate);
        }
        if let Some(candidate) = self.definition_candidate(corpus)? {
            candidates.push(candidate);
        }
        if let Some(candidate) = self.dependency_candidate(corpus)? {
            candidates.push(candidate);
        }
        if let Some(candidate) = self.schedule_candidate(corpus)? {
            candidates.push(candidate);
        }
        Ok(candidates)
    }

    /// Derives the content-addressed id for one generated change.
    fn derive_id(
        &self,
        kind: ChangeKind,
        incumbent: &WorkflowVersionId,
        provenance: &CandidateProvenance,
    ) -> Result<CandidateId, WorkflowEvolutionError> {
        let projection = provenance.projection(kind, incumbent);
        let digest = ContentDigest::of(&projection)?;
        CandidateId::derive(&self.policy.id_prefix, kind.key(), &digest)
    }

    /// Rule: escalations or recovery records propose scoping binding
    /// resolution to the environments with observed service.
    ///
    /// The successor policy keeps human fallback forbidden: restricting
    /// scope is a conservative refinement, never a permission widening.
    fn recovery_candidate(
        &self,
        corpus: &EvidenceCorpus,
    ) -> Result<Option<ImprovementCandidate>, WorkflowEvolutionError> {
        let recoveries = corpus.count_kind(EvidenceKind::Recovery);
        if corpus.escalations == 0 && recoveries == 0 {
            return Ok(None);
        }
        if corpus.observed_environments.is_empty() {
            return Ok(None);
        }
        let environments: Vec<ExecutionEnvironment> =
            corpus.observed_environments.iter().copied().collect();
        let provenance = self.provenance(corpus);
        let id = self.derive_id(
            ChangeKind::RecoveryPolicy,
            &corpus.incumbent.version_id,
            &provenance,
        )?;
        Ok(Some(ImprovementCandidate {
            id,
            incumbent: corpus.incumbent.clone(),
            change: ProposedChange::Recovery {
                policy: BindingPolicy {
                    environments: EnvironmentScope::Only(environments),
                    human_fallback: HumanFallbackPolicy::Forbidden,
                },
            },
            rationale: format!(
                "{} escalation(s) and {} recovery record(s) across {} run(s): scope binding \
                 resolution to the {} observed environment(s) with successful service; human \
                 fallback stays forbidden",
                corpus.escalations,
                recoveries,
                corpus.runs.len(),
                corpus.observed_environments.len()
            ),
            provenance,
        }))
    }

    /// Rule: observation evidence proposes restating the incumbent step
    /// bindings with purposes that cite the evidence stream.
    ///
    /// The declared capabilities never change in this rule: only the
    /// human-facing purpose text, which cites how many observations back
    /// the step.
    fn bindings_candidate(
        &self,
        corpus: &EvidenceCorpus,
    ) -> Result<Option<ImprovementCandidate>, WorkflowEvolutionError> {
        let observations = corpus.count_kind(EvidenceKind::Observation);
        if observations == 0 {
            return Ok(None);
        }
        let mut bindings = BTreeMap::new();
        for (node_id, node) in &corpus.definition.ir.nodes {
            if let WorkflowIrNode::Step(step) = node {
                if step.capabilities.is_empty() {
                    continue;
                }
                let requirements: Vec<CapabilityRequirement> = step
                    .capabilities
                    .iter()
                    .map(|requirement| {
                        let purpose = match &requirement.purpose {
                            Some(base) => {
                                format!("{base} (evidence: {observations} observations)")
                            }
                            None => format!("evidence: {observations} observations"),
                        };
                        CapabilityRequirement {
                            capability: requirement.capability.clone(),
                            purpose: Some(purpose),
                        }
                    })
                    .collect();
                bindings.insert(node_id.clone(), requirements);
            }
        }
        if bindings.is_empty() {
            return Ok(None);
        }
        let provenance = self.provenance(corpus);
        let id = self.derive_id(
            ChangeKind::CapabilityBinding,
            &corpus.incumbent.version_id,
            &provenance,
        )?;
        Ok(Some(ImprovementCandidate {
            id,
            incumbent: corpus.incumbent.clone(),
            change: ProposedChange::Bindings { bindings },
            rationale: format!(
                "{observations} observation reference(s) across {} run(s): restate the step \
                 bindings with purposes citing the backing observations",
                corpus.runs.len()
            ),
            provenance,
        }))
    }

    /// Rule: trace evidence proposes refreshing the definition's
    /// human-facing description from the recorded traces.
    ///
    /// Only the description changes; the graph, bindings, and dependencies
    /// are carried over untouched.
    fn definition_candidate(
        &self,
        corpus: &EvidenceCorpus,
    ) -> Result<Option<ImprovementCandidate>, WorkflowEvolutionError> {
        let traces = corpus.count_kind(EvidenceKind::Trace);
        if traces == 0 {
            return Ok(None);
        }
        let mut definition = corpus.definition.clone();
        definition.description = Some(format!(
            "governed evolution from {} recorded trace(s) and {} run(s)",
            traces,
            corpus.runs.len()
        ));
        let provenance = self.provenance(corpus);
        let id = self.derive_id(
            ChangeKind::DefinitionDelta,
            &corpus.incumbent.version_id,
            &provenance,
        )?;
        Ok(Some(ImprovementCandidate {
            id,
            incumbent: corpus.incumbent.clone(),
            change: ProposedChange::Definition { definition },
            rationale: format!(
                "{traces} trace reference(s) across {} run(s): refresh the definition \
                 description from the recorded traces",
                corpus.runs.len()
            ),
            provenance,
        }))
    }

    /// Rule: lock entries beyond the declared dependencies propose
    /// minimizing the lock to exactly the declared set.
    ///
    /// The incumbent definition declares which dependencies the workflow
    /// actually uses; resolved entries beyond that declaration are unused
    /// resolution weight.
    fn dependency_candidate(
        &self,
        corpus: &EvidenceCorpus,
    ) -> Result<Option<ImprovementCandidate>, WorkflowEvolutionError> {
        let declared_keys = declared_lock_keys(&corpus.definition);
        let excess: Vec<&DependencyKey> = corpus
            .dependency_lock
            .entries
            .keys()
            .filter(|key| !declared_keys.contains(key))
            .collect();
        if excess.is_empty() {
            return Ok(None);
        }
        let mut lock = DependencyLock::default();
        for (key, entry) in &corpus.dependency_lock.entries {
            if declared_keys.contains(key) {
                lock.insert(entry.clone());
            }
        }
        let provenance = self.provenance(corpus);
        let id = self.derive_id(
            ChangeKind::DependencyChoice,
            &corpus.incumbent.version_id,
            &provenance,
        )?;
        Ok(Some(ImprovementCandidate {
            id,
            incumbent: corpus.incumbent.clone(),
            change: ProposedChange::Dependency { lock },
            rationale: format!(
                "{} resolved lock entr(ies) beyond the {} declared dependenc(ies): minimize the \
                 lock to the declared set",
                excess.len(),
                declared_keys.len()
            ),
            provenance,
        }))
    }

    /// Rule: a recorded schedule and fully healthy runs propose doubling
    /// the schedule period (halving the cadence).
    ///
    /// Healthy means every recorded run settled `Succeeded` with zero
    /// escalations. The halved period must stay positive, so periods of
    /// one millisecond are left alone.
    fn schedule_candidate(
        &self,
        corpus: &EvidenceCorpus,
    ) -> Result<Option<ImprovementCandidate>, WorkflowEvolutionError> {
        let Some(ScheduleSpec::Every { period_ms }) = corpus.observed_schedule else {
            return Ok(None);
        };
        if period_ms < 2 || corpus.escalations != 0 {
            return Ok(None);
        }
        let healthy = !corpus.runs.is_empty()
            && corpus
                .runs
                .iter()
                .all(|run| run.status == WorkflowInstanceStatus::Succeeded);
        if !healthy {
            return Ok(None);
        }
        let provenance = self.provenance(corpus);
        let id = self.derive_id(
            ChangeKind::ScheduleTuning,
            &corpus.incumbent.version_id,
            &provenance,
        )?;
        Ok(Some(ImprovementCandidate {
            id,
            incumbent: corpus.incumbent.clone(),
            change: ProposedChange::Schedule {
                spec: ScheduleSpec::Every {
                    period_ms: period_ms / 2,
                },
            },
            rationale: format!(
                "{} healthy run(s) at the observed every-{}ms schedule: halve the cadence \
                 (period {}ms)",
                corpus.runs.len(),
                period_ms,
                period_ms / 2
            ),
            provenance,
        }))
    }

    /// The full provenance snapshot shared by generated candidates.
    fn provenance(&self, corpus: &EvidenceCorpus) -> CandidateProvenance {
        CandidateProvenance {
            evidence: corpus.references.clone(),
            runs: corpus.runs.clone(),
        }
    }
}

impl Default for CandidateGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// The lock keys declared by a definition's dependency sections.
fn declared_lock_keys(definition: &WorkflowDefinition) -> BTreeSet<DependencyKey> {
    let mut keys = BTreeSet::new();
    for skill in &definition.dependencies.skills {
        keys.insert(DependencyKey::Skill {
            skill: skill.skill.clone(),
        });
    }
    for plugin in &definition.dependencies.plugins {
        keys.insert(DependencyKey::Plugin {
            plugin: plugin.plugin.clone(),
        });
    }
    for mcp in &definition.dependencies.mcp {
        keys.insert(DependencyKey::Mcp {
            capability: mcp.capability.clone(),
        });
    }
    for subworkflow in &definition.dependencies.subworkflows {
        keys.insert(DependencyKey::Subworkflow {
            dependency_id: subworkflow.dependency_id.clone(),
        });
    }
    keys
}

#[cfg(test)]
#[path = "generate_tests.rs"]
mod tests;
