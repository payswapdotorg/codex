//! PACK-005: semantic composition contracts.
//!
//! Composition is the governed way two pack revisions produce a THIRD,
//! new, candidate state:
//!
//! ```text
//! Pack A + Pack B → Candidate Pack State
//! ```
//!
//! never a mutation of either parent. Every function in this module takes
//! parents by shared reference and returns fresh records; the parents'
//! identities and digests are unchanged by construction, and the composed
//! result records both parents in a [`CompositionRecord`] that participates
//! in the revision identity, so composition provenance is tamper-evident.
//!
//! Per the frozen architecture §10, composition supports:
//!
//! - [`CompositionRelation::Specialization`]: the overlay specializes the
//!   base; the overlay's mission governs and its refinements win.
//! - [`CompositionRelation::Orthogonal`]: side-by-side merge; the missions
//!   merge by union under a caller-supplied composed mission header.
//! - Contextual activation ([`ContextualActivation`]): typed rules binding
//!   composed system-state targets to activation scopes. Absent rules mean
//!   unconditional (always) activation — the PACK-002 default. Activation
//!   is a contract (data), never an execution engine.
//!
//! Composition must resolve identity, dependency versions, capability and
//! policy conflicts, provenance, and activation — and unresolved semantic
//! conflicts are explicit typed errors
//! ([`crate::PackContractError::CompositionConflict`]). The runtime must
//! never resolve material pack conflicts by undocumented guesswork; this
//! module refuses them instead.
//!
//! Authority is unchanged by composition: composing packs grants no
//! workflow-transition, credential, or evidence authority, and the composed
//! result is always a *candidate* — promotion remains a control-plane
//! decision through [`crate::PromotedPackState::promote`].
//!
//! This is a contracts-only module: pure functions over immutable records,
//! no storage, no controllers, no engines.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::WorkflowVersionId;
use serde::Deserialize;
use serde::Serialize;

use crate::CandidatePackState;
use crate::CapabilityRef;
use crate::ContextModel;
use crate::Mission;
use crate::MissionAuthor;
use crate::MissionId;
use crate::MissionStatement;
use crate::PackAssurancePolicy;
use crate::PackConstitution;
use crate::PackContractError;
use crate::PackDependencyLock;
use crate::PackId;
use crate::PackPolicy;
use crate::PackPolicyId;
use crate::PackPolicySet;
use crate::PackProvenance;
use crate::PackRevisionContent;
use crate::PackRevisionIdentity;
use crate::PackSystemState;
use crate::ParentRevision;
use crate::PolicyRef;
use crate::ValueModel;
use crate::WorkflowVersionRef;

/// How the two parents relate semantically.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum CompositionRelation {
    /// The overlay specializes the base: the overlay's mission governs
    /// verbatim, and for descriptive collisions (roles, constraints) the
    /// overlay's refinement wins.
    Specialization,
    /// Neither parent specializes the other: the missions merge by union
    /// under a caller-supplied composed mission header, and for descriptive
    /// collisions the base's text wins (the overlay adds, it does not
    /// rewrite).
    Orthogonal,
}

/// How activation-rule collisions between the parents are treated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum ActivationMergeMode {
    /// Contradictory activation for the same target (one parent always, the
    /// other conditional; or two different conditions) is an explicit
    /// [`CompositionConflictDetail::ActivationIncompatible`] error. The
    /// default, safest mode: conflicts surface instead of silently
    /// resolving.
    RequireCompatible,
    /// Contradictory activation merges predictably: an unconditional rule
    /// (always) subsumes a conditional one, and two different conditions
    /// for the same target coexist (the target is active when either
    /// condition holds — an OR over the recorded rules).
    Union,
}

/// The explicit, caller-stated composition strategy.
///
/// There are no hidden defaults: the strategy is recorded verbatim in the
/// [`CompositionRecord`] and participates in the composed revision
/// identity, so two compositions of the same parents under different
/// strategies are distinct governed results.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompositionStrategy {
    /// How activation-rule collisions merge.
    pub activation_merge: ActivationMergeMode,
}

impl CompositionStrategy {
    /// The conservative default strategy: activation conflicts are errors.
    pub const fn strict() -> Self {
        Self {
            activation_merge: ActivationMergeMode::RequireCompatible,
        }
    }
}

/// A composed system-state element that contextual activation binds.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum ActivationTarget {
    /// A pinned workflow version in the composed system state.
    WorkflowVersion(WorkflowVersionId),
    /// A required capability in the composed system state.
    Capability(CapabilityId),
}

impl ActivationTarget {
    /// The target as stable display text (for reports and errors).
    pub fn display(&self) -> String {
        match self {
            Self::WorkflowVersion(id) => format!("workflow-version:{id}"),
            Self::Capability(id) => format!("capability:{}", id.as_ref()),
        }
    }
}

/// When a composed element is active.
///
/// `Always` is the PACK-002 default for every reference and needs no rule;
/// rules exist to express conditionality.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum ActivationScope {
    /// Unconditionally active (the default; recorded only when a parent
    /// narrowed a previously conditional rule back to always).
    Always,
    /// Active when the composed mission's context model carries this exact
    /// context note.
    WhenContextNote(MissionStatement),
}

impl ActivationScope {
    /// The scope as stable display text (for reports and errors).
    pub fn display(&self) -> String {
        match self {
            Self::Always => "always".to_owned(),
            Self::WhenContextNote(note) => format!("context-note:{}", note.as_str()),
        }
    }
}

/// One contextual-activation rule: a target plus the scope it is active in.
///
/// Rules are contracts (data). They never execute anything and never grant
/// authority; an execution environment reading the composed candidate is
/// told, exactly and typed, under which context a composed element is part
/// of the governed system state.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextualActivation {
    /// The composed element the rule binds.
    pub target: ActivationTarget,
    /// The scope in which the element is active.
    pub scope: ActivationScope,
}

/// The caller-supplied header of an orthogonally composed mission.
///
/// Mission authority is user authority: composing two user-authored
/// missions cannot silently inherit either parent's authorship, so the
/// composing principal states the composed mission's identity, statement,
/// and author explicitly.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComposedMissionHeader {
    /// The composed mission's lineage identity.
    pub id: MissionId,
    /// The composed mission's statement.
    pub statement: MissionStatement,
    /// Who authored the composed mission content.
    pub author: MissionAuthor,
}

/// The composed pack's lineage: which pack lineage the candidate belongs to
/// and at which semantic version. Composition never invents lineage; the
/// composing principal states it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComposedLineage {
    /// The pack lineage the composed candidate belongs to.
    pub pack_id: PackId,
    /// The semantic version of the composed candidate.
    pub semantic_version: SemanticVersion,
}

/// The full two-parent provenance of a composed revision.
///
/// Carried by [`PackRevisionContent::composition`] and digested into the
/// revision identity (via `composition_digest` in
/// [`crate::PackRevisionIdentity`]), so a composition can never hide: the
/// same governed content composed from different parents, under a different
/// relation or strategy, is a different revision.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompositionRecord {
    /// The base parent's full revision identity tuple.
    pub base: PackRevisionIdentity,
    /// The overlay parent's full revision identity tuple.
    pub overlay: PackRevisionIdentity,
    /// How the parents relate.
    pub relation: CompositionRelation,
    /// The strategy the composition ran under.
    pub strategy: CompositionStrategy,
    /// The merged contextual-activation rules (never-empty scopes only;
    /// always-active is the default and is not recorded).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub activations: Vec<ContextualActivation>,
}

impl CompositionRecord {
    /// Validates the record's structural invariants.
    ///
    /// Exact-duplicate activation rules are rejected (an OR over identical
    /// rules is one rule); two rules sharing a target but differing in
    /// scope are legal Union-mode output.
    pub fn validate(&self) -> Result<(), PackContractError> {
        let mut seen = BTreeSet::new();
        for activation in &self.activations {
            if !seen.insert(activation) {
                return Err(PackContractError::CompositionConflict {
                    detail: CompositionConflictDetail::DuplicateActivation {
                        target: activation.target.display(),
                    },
                });
            }
        }
        Ok(())
    }

    /// Canonical content digest of the composition record.
    ///
    /// This is the digest pinned as `composition_digest` in the revision
    /// identity tuple of a composed revision.
    pub fn digest(&self) -> Result<ContentDigest, PackContractError> {
        self.validate()?;
        Ok(ContentDigest::of(self)?)
    }
}

/// A verified, borrowed snapshot of one parent revision.
///
/// Built exclusively from a [`crate::CandidatePackState`] or
/// [`crate::PromotedPackState`] whose integrity has been verified, so
/// composition inputs are always trustworthy governed records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackRevisionView {
    /// The parent's full revision identity tuple.
    pub identity: PackRevisionIdentity,
    /// The parent's governed content.
    pub content: PackRevisionContent,
}

impl PackRevisionView {
    /// Snapshots a verified candidate revision.
    pub fn of_candidate(candidate: &CandidatePackState) -> Result<Self, PackContractError> {
        candidate.verify_integrity()?;
        Ok(Self {
            identity: PackRevisionIdentity {
                pack: candidate.pack_id.clone(),
                semantic_version: candidate.semantic_version.clone(),
                system_state_digest: candidate.system_state_digest.clone(),
                mission_digest: candidate.mission_digest.clone(),
                policy_digest: candidate.policy_digest.clone(),
                dependency_lock_digest: candidate.dependency_lock_digest.clone(),
                parent_revision: candidate.parent.as_ref().map(|p| p.revision.clone()),
                composition_digest: candidate.composition_digest.clone(),
            },
            content: PackRevisionContent {
                pack_id: candidate.pack_id.clone(),
                semantic_version: candidate.semantic_version.clone(),
                mission: candidate.mission.clone(),
                policy_set: candidate.policy_set.clone(),
                dependency_lock: candidate.dependency_lock.clone(),
                system_state: candidate.system_state.clone(),
                composition: candidate.composition.clone(),
            },
        })
    }

    /// Snapshots a verified promoted revision.
    pub fn of_promoted(promoted: &crate::PromotedPackState) -> Result<Self, PackContractError> {
        promoted.verify_integrity()?;
        Ok(Self {
            identity: PackRevisionIdentity {
                pack: promoted.pack_id.clone(),
                semantic_version: promoted.semantic_version.clone(),
                system_state_digest: promoted.system_state_digest.clone(),
                mission_digest: promoted.mission_digest.clone(),
                policy_digest: promoted.policy_digest.clone(),
                dependency_lock_digest: promoted.dependency_lock_digest.clone(),
                parent_revision: promoted.parent.as_ref().map(|p| p.revision.clone()),
                composition_digest: promoted.composition_digest.clone(),
            },
            content: PackRevisionContent {
                pack_id: promoted.pack_id.clone(),
                semantic_version: promoted.semantic_version.clone(),
                mission: promoted.mission.clone(),
                policy_set: promoted.policy_set.clone(),
                dependency_lock: promoted.dependency_lock.clone(),
                system_state: promoted.system_state.clone(),
                composition: promoted.composition.clone(),
            },
        })
    }
}

/// The composed-pack request: parents, relation, strategy, lineage, and the
/// optional pieces (mission header for orthogonal merges; assurance
/// policies to compose).
pub struct ComposePackRequest<'a> {
    /// The base parent (the revision being specialized or merged onto).
    pub base: &'a PackRevisionView,
    /// The overlay parent (the specialization or the additive merge).
    pub overlay: &'a PackRevisionView,
    /// How the parents relate.
    pub relation: CompositionRelation,
    /// The composition strategy.
    pub strategy: CompositionStrategy,
    /// The composed candidate's lineage.
    pub lineage: ComposedLineage,
    /// The composed mission header. Required for
    /// [`CompositionRelation::Orthogonal`]; must be absent for
    /// [`CompositionRelation::Specialization`] (the overlay's mission
    /// governs verbatim).
    pub mission_header: Option<ComposedMissionHeader>,
    /// The base parent's assurance policy, when the caller holds it.
    pub base_assurance: Option<&'a PackAssurancePolicy>,
    /// The overlay parent's assurance policy, when the caller holds it.
    pub overlay_assurance: Option<&'a PackAssurancePolicy>,
}

/// What composition resolved, exactly.
///
/// The report is inspectable output, not authority: it states what merged,
/// what unioned, and what activated conditionally, so a reviewing principal
/// can see precisely what the composed candidate contains.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompositionReport {
    /// The relation the composition ran under.
    pub relation: CompositionRelation,
    /// Distinct constitution rules after the union (both parents; a
    /// composition can add constraints, never drop them).
    pub constitution_rules: usize,
    /// Distinct governing policies after the union.
    pub governing_policies: usize,
    /// Dependency-lock entries after the union.
    pub dependency_entries: usize,
    /// Pinned workflow versions after the union.
    pub workflow_versions: usize,
    /// Required capabilities after the union.
    pub capabilities: usize,
    /// Whether the parents' assurance policies were composed into one.
    pub assurance_composed: bool,
    /// The conditional activation rules of the composed state (always-active
    /// is the default and is not listed).
    pub activations: Vec<ContextualActivation>,
}

/// The result of a successful composition.
///
/// `content` is the governed bundle carrying the
/// [`CompositionRecord`] provenance; `composed_assurance` is the composed
/// assurance policy (also pinned in the composed system state's policy
/// references); `report` states what merged. Use
/// [`ComposedPack::into_candidate`] to flow the result into the PACK-002
/// governed lifecycle.
#[derive(Debug)]
pub struct ComposedPack {
    /// The composed governed content (carries composition provenance).
    pub content: PackRevisionContent,
    /// The composed assurance policy, when assurance inputs were given.
    pub composed_assurance: Option<PackAssurancePolicy>,
    /// What composition resolved.
    pub report: CompositionReport,
}

impl ComposedPack {
    /// Flows the composed content into the governed lifecycle as a new
    /// candidate revision.
    ///
    /// The single recorded parent is the base revision (lineage follows the
    /// base); the two-parent truth lives in the composition record. The
    /// provenance must name the identical base revision
    /// (for example `PackProvenance::child_of(base_revision_id, producer)`).
    pub fn into_candidate(
        self,
        parent: ParentRevision,
        provenance: PackProvenance,
    ) -> Result<CandidatePackState, PackContractError> {
        CandidatePackState::propose(self.content, parent, provenance)
    }
}

/// Composes two parent revisions into a new governed candidate state.
///
/// The critical property: `Pack A + Pack B → Candidate Pack State`. Both
/// parents are read by shared reference and never modified; the result is a
/// fresh [`PackRevisionContent`] whose composition record names both
/// parents, the relation, the strategy, and the merged activation rules.
///
/// Unresolved semantic conflicts — a re-pinned dependency, a policy whose
/// content disagrees with its identity, incompatible activation,
/// assurance pins that cannot compose — are explicit
/// [`crate::PackContractError::CompositionConflict`] errors. Nothing is
/// silently resolved.
pub fn compose_pack(request: ComposePackRequest<'_>) -> Result<ComposedPack, PackContractError> {
    let ComposePackRequest {
        base,
        overlay,
        relation,
        strategy,
        lineage,
        mission_header,
        base_assurance,
        overlay_assurance,
    } = request;

    // Inputs must be valid governed content (the views already verified
    // record integrity at construction; content validation is cheap and
    // keeps this function honest for hand-built views).
    base.content.validate()?;
    overlay.content.validate()?;

    // --- mission ---------------------------------------------------------
    let mission = compose_mission(base, overlay, relation, mission_header)?;

    // --- constitution: union, never weaker --------------------------------
    let mut rules = base.content.policy_set.constitution.rules.clone();
    for overlay_rule in &overlay.content.policy_set.constitution.rules {
        if !rules.contains(overlay_rule) {
            rules.push(overlay_rule.clone());
        }
    }
    let constitution = PackConstitution::new(rules)?;

    // --- governing policies: union by content-derived identity -------------
    let mut policies_by_id: BTreeMap<PackPolicyId, PackPolicy> = BTreeMap::new();
    let mut policies: Vec<PackPolicy> = Vec::new();
    for parent in [base, overlay] {
        for policy in &parent.content.policy_set.policies {
            let id = policy.policy_id()?;
            match policies_by_id.get(&id) {
                Some(existing) if existing != policy => {
                    return Err(PackContractError::CompositionConflict {
                        detail: CompositionConflictDetail::PolicyContentMismatch {
                            policy_id: id.digest().to_string(),
                        },
                    });
                }
                Some(_) => {}
                None => {
                    policies_by_id.insert(id, policy.clone());
                    policies.push(policy.clone());
                }
            }
        }
    }

    // --- assurance: delegate to PACK-003 composition ------------------------
    let composed_assurance = match (base_assurance, overlay_assurance) {
        (Some(base_policy), Some(overlay_policy)) => {
            Some(base_policy.compose(overlay_policy).map_err(|err| {
                PackContractError::CompositionConflict {
                    detail: CompositionConflictDetail::AssuranceIncompatible {
                        reason: err.to_string(),
                    },
                }
            })?)
        }
        (Some(base_policy), None) => Some(base_policy.clone()),
        (None, Some(overlay_policy)) => Some(overlay_policy.clone()),
        (None, None) => None,
    };

    // --- policy set ---------------------------------------------------------
    let policy_set = PackPolicySet::new(constitution, policies)?;

    // --- dependency lock: union; re-pins are explicit conflicts --------------
    let dependency_lock = compose_locks(base, overlay)?;

    // --- system state: union of references -----------------------------------
    let system_state = compose_system_states(base, overlay, relation, composed_assurance.as_ref())?;

    // --- contextual activation ------------------------------------------------
    let activations = compose_activations(base, overlay, strategy, &system_state, &mission)?;

    // --- assemble the governed content -----------------------------------------
    let record = CompositionRecord {
        base: base.identity.clone(),
        overlay: overlay.identity.clone(),
        relation,
        strategy,
        activations,
    };
    let content = PackRevisionContent {
        pack_id: lineage.pack_id,
        semantic_version: lineage.semantic_version,
        mission,
        policy_set,
        dependency_lock,
        system_state,
        composition: Some(record),
    };
    content.validate()?;

    let report = CompositionReport {
        relation,
        constitution_rules: content.policy_set.constitution.rules.len(),
        governing_policies: content.policy_set.policies.len(),
        dependency_entries: content.dependency_lock.entries.len(),
        workflow_versions: content.system_state.workflow_version_refs.len(),
        capabilities: content.system_state.capability_refs.len(),
        assurance_composed: composed_assurance.is_some(),
        activations: content
            .composition
            .as_ref()
            .map(|record| record.activations.clone())
            .unwrap_or_default(),
    };

    Ok(ComposedPack {
        content,
        composed_assurance,
        report,
    })
}

/// Merges the parents' missions per the relation.
fn compose_mission(
    base: &PackRevisionView,
    overlay: &PackRevisionView,
    relation: CompositionRelation,
    header: Option<ComposedMissionHeader>,
) -> Result<Mission, PackContractError> {
    match relation {
        CompositionRelation::Specialization => {
            if header.is_some() {
                return Err(PackContractError::CompositionConflict {
                    detail: CompositionConflictDetail::MissionHeaderUnexpected,
                });
            }
            // The overlay's mission governs verbatim; the base mission is
            // retained in the composition provenance (identity tuple).
            Ok(overlay.content.mission.clone())
        }
        CompositionRelation::Orthogonal => {
            let header = header.ok_or(PackContractError::CompositionConflict {
                detail: CompositionConflictDetail::MissionHeaderRequired,
            })?;
            let base_mission = &base.content.mission;
            let overlay_mission = &overlay.content.mission;
            Ok(Mission {
                id: header.id,
                statement: header.statement,
                author: header.author,
                value_model: ValueModel {
                    objectives: merge_dedup(
                        base_mission.value_model.objectives.clone(),
                        overlay_mission.value_model.objectives.clone(),
                    ),
                },
                context_model: ContextModel {
                    notes: merge_dedup(
                        base_mission.context_model.notes.clone(),
                        overlay_mission.context_model.notes.clone(),
                    ),
                },
                hard_constraints: merge_dedup(
                    base_mission.hard_constraints.clone(),
                    overlay_mission.hard_constraints.clone(),
                ),
                preferences: merge_dedup(
                    base_mission.preferences.clone(),
                    overlay_mission.preferences.clone(),
                ),
                success_measures: merge_dedup(
                    base_mission.success_measures.clone(),
                    overlay_mission.success_measures.clone(),
                ),
            })
        }
    }
}

/// Base elements first, then overlay elements not already present.
fn merge_dedup<T: PartialEq + Eq>(mut base: Vec<T>, overlay: Vec<T>) -> Vec<T> {
    for element in overlay {
        if !base.contains(&element) {
            base.push(element);
        }
    }
    base
}

/// Unions the parents' dependency locks; a re-pinned dependency is an
/// explicit conflict, never a silent re-resolution.
fn compose_locks(
    base: &PackRevisionView,
    overlay: &PackRevisionView,
) -> Result<PackDependencyLock, PackContractError> {
    let mut entries = base.content.dependency_lock.entries.clone();
    for (key, overlay_entry) in &overlay.content.dependency_lock.entries {
        match entries.get(key) {
            Some(base_entry) if base_entry != overlay_entry => {
                return Err(PackContractError::CompositionConflict {
                    detail: CompositionConflictDetail::DependencyRePin {
                        key: key.as_key(),
                        base: format!("{:?}", base_entry.resolved),
                        overlay: format!("{:?}", overlay_entry.resolved),
                    },
                });
            }
            Some(_) => {}
            None => {
                entries.insert(key.clone(), overlay_entry.clone());
            }
        }
    }
    Ok(PackDependencyLock { entries })
}

/// Unions the parents' system states, pinning the composed assurance policy
/// when one was produced.
fn compose_system_states(
    base: &PackRevisionView,
    overlay: &PackRevisionView,
    relation: CompositionRelation,
    composed_assurance: Option<&PackAssurancePolicy>,
) -> Result<PackSystemState, PackContractError> {
    // Workflow versions: union by identity. Roles are descriptive; on
    // collision the overlay's role wins under Specialization (the overlay
    // refines) and the base's wins under Orthogonal (the overlay adds).
    let mut workflow_version_refs: Vec<WorkflowVersionRef> = Vec::new();
    let mut workflow_seen: BTreeMap<&WorkflowVersionId, ()> = BTreeMap::new();
    for parent in [base, overlay] {
        for reference in &parent.content.system_state.workflow_version_refs {
            if workflow_seen
                .insert(&reference.workflow_version_id, ())
                .is_some()
            {
                continue;
            }
            workflow_version_refs.push(reference.clone());
        }
    }
    if relation == CompositionRelation::Specialization {
        for overlay_ref in &overlay.content.system_state.workflow_version_refs {
            if let Some(slot) = workflow_version_refs
                .iter_mut()
                .find(|r| r.workflow_version_id == overlay_ref.workflow_version_id)
            {
                slot.role = overlay_ref.role.clone();
            }
        }
    }

    // Capabilities: union by identity with the same role/constraint rule.
    let mut capability_refs: Vec<CapabilityRef> = Vec::new();
    let mut capability_seen: BTreeMap<&CapabilityId, ()> = BTreeMap::new();
    for parent in [base, overlay] {
        for reference in &parent.content.system_state.capability_refs {
            if capability_seen.insert(&reference.capability, ()).is_some() {
                continue;
            }
            capability_refs.push(reference.clone());
        }
    }
    if relation == CompositionRelation::Specialization {
        for overlay_ref in &overlay.content.system_state.capability_refs {
            if let Some(slot) = capability_refs
                .iter_mut()
                .find(|r| r.capability == overlay_ref.capability)
            {
                slot.constraint = overlay_ref.constraint.clone();
            }
        }
    }

    // Policy references: union by identity; the same identity validated
    // against different content is an explicit conflict.
    let mut policy_refs: Vec<PolicyRef> = Vec::new();
    let mut policy_seen: BTreeMap<&PackPolicyId, &PolicyRef> = BTreeMap::new();
    for parent in [base, overlay] {
        for reference in &parent.content.system_state.policy_refs {
            match policy_seen.get(&reference.policy_id) {
                Some(existing) => {
                    if existing.validated_content_digest != reference.validated_content_digest {
                        return Err(PackContractError::CompositionConflict {
                            detail: CompositionConflictDetail::PolicyContentMismatch {
                                policy_id: reference.policy_id.digest().to_string(),
                            },
                        });
                    }
                }
                None => {
                    policy_seen.insert(&reference.policy_id, reference);
                    policy_refs.push(reference.clone());
                }
            }
        }
    }

    // The composed assurance policy is pinned by the composed state.
    if let Some(assurance) = composed_assurance {
        let policy_id = assurance.policy_id()?;
        if !policy_seen.contains_key(&policy_id) {
            let validated_digest = policy_id.digest().clone();
            policy_refs.push(PolicyRef::validated_against(policy_id, validated_digest));
        }
    }

    // Evaluations and evidence: opaque content addresses. An artifact both
    // parents assessed or evidenced is referenced once by the union (the
    // same content address is the same record), never duplicated.
    let evaluation_refs: Vec<crate::EvaluationRef> = base
        .content
        .system_state
        .evaluation_refs
        .iter()
        .chain(overlay.content.system_state.evaluation_refs.iter())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let evidence_refs: Vec<crate::EvidenceRef> = base
        .content
        .system_state
        .evidence_refs
        .iter()
        .chain(overlay.content.system_state.evidence_refs.iter())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    PackSystemState::new(
        workflow_version_refs,
        capability_refs,
        policy_refs,
        evaluation_refs,
        evidence_refs,
        /*rollback_checkpoint*/ None,
    )
}

/// Merges the parents' contextual-activation rules over the merged state.
fn compose_activations(
    base: &PackRevisionView,
    overlay: &PackRevisionView,
    strategy: CompositionStrategy,
    merged_state: &PackSystemState,
    merged_mission: &Mission,
) -> Result<Vec<ContextualActivation>, PackContractError> {
    // Collect each parent's explicit rules (duplicate targets within one
    // parent were already rejected by record validation).
    let base_rules = parent_rules(base);
    let overlay_rules = parent_rules(overlay);

    // Every target in the merged state, plus any explicitly-ruled target.
    let mut targets: BTreeSet<ActivationTarget> = BTreeSet::new();
    for reference in &merged_state.workflow_version_refs {
        targets.insert(ActivationTarget::WorkflowVersion(
            reference.workflow_version_id.clone(),
        ));
    }
    for reference in &merged_state.capability_refs {
        targets.insert(ActivationTarget::Capability(reference.capability.clone()));
    }
    targets.extend(base_rules.keys().cloned());
    targets.extend(overlay_rules.keys().cloned());

    let mut merged: Vec<ContextualActivation> = Vec::new();
    for target in targets {
        let base_scope = effective_scope(base, &base_rules, &target);
        let overlay_scope = effective_scope(overlay, &overlay_rules, &target);
        let scopes = match (base_scope, overlay_scope) {
            (Some(a), Some(b)) => match merge_scopes(&target, a, b, strategy) {
                MergeOutcome::Conflict(detail) => {
                    return Err(PackContractError::CompositionConflict { detail });
                }
                MergeOutcome::Scopes(scopes) => scopes,
            },
            (Some(a), None) => vec![a],
            (None, Some(b)) => vec![b],
            (None, None) => continue,
        };
        for scope in scopes {
            if scope != ActivationScope::Always {
                merged.push(ContextualActivation {
                    target: target.clone(),
                    scope,
                });
            }
        }
    }

    // A rule for a target that is not part of the merged state is an
    // unresolved conflict, not a silent drop.
    for activation in &merged {
        let present = match &activation.target {
            ActivationTarget::WorkflowVersion(id) => merged_state
                .workflow_version_refs
                .iter()
                .any(|r| &r.workflow_version_id == id),
            ActivationTarget::Capability(id) => merged_state
                .capability_refs
                .iter()
                .any(|r| &r.capability == id),
        };
        if !present {
            return Err(PackContractError::CompositionConflict {
                detail: CompositionConflictDetail::ActivationTargetMissing {
                    target: activation.target.display(),
                },
            });
        }
    }

    // A conditional rule whose context note is not part of the composed
    // mission's context model is dangling, not silently dropped.
    for activation in &merged {
        if let ActivationScope::WhenContextNote(note) = &activation.scope {
            let known = merged_mission
                .context_model
                .notes
                .iter()
                .any(|context_note| &context_note.statement == note);
            if !known {
                return Err(PackContractError::CompositionConflict {
                    detail: CompositionConflictDetail::DanglingActivationContext {
                        note: note.as_str().to_owned(),
                    },
                });
            }
        }
    }

    merged.sort();
    Ok(merged)
}

/// A parent's explicit activation rules, keyed by target.
fn parent_rules(parent: &PackRevisionView) -> BTreeMap<ActivationTarget, ActivationScope> {
    parent
        .content
        .composition
        .as_ref()
        .map(|record| {
            record
                .activations
                .iter()
                .map(|activation| (activation.target.clone(), activation.scope.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// A parent's effective scope for a target: its explicit rule, else `Always`
/// when its own state references the target, else no opinion.
fn effective_scope(
    parent: &PackRevisionView,
    rules: &BTreeMap<ActivationTarget, ActivationScope>,
    target: &ActivationTarget,
) -> Option<ActivationScope> {
    if let Some(scope) = rules.get(target) {
        return Some(scope.clone());
    }
    let referenced = match target {
        ActivationTarget::WorkflowVersion(id) => parent
            .content
            .system_state
            .workflow_version_refs
            .iter()
            .any(|r| &r.workflow_version_id == id),
        ActivationTarget::Capability(id) => parent
            .content
            .system_state
            .capability_refs
            .iter()
            .any(|r| &r.capability == id),
    };
    if referenced {
        Some(ActivationScope::Always)
    } else {
        None
    }
}

/// Outcome of merging two scopes for one target.
enum MergeOutcome {
    /// The scopes compose into one or more (Union mode may keep both
    /// conditions).
    Scopes(Vec<ActivationScope>),
    /// The scopes conflict (RequireCompatible mode).
    Conflict(CompositionConflictDetail),
}

fn merge_scopes(
    target: &ActivationTarget,
    base: ActivationScope,
    overlay: ActivationScope,
    strategy: CompositionStrategy,
) -> MergeOutcome {
    use ActivationMergeMode::RequireCompatible;
    use ActivationMergeMode::Union;
    use ActivationScope::Always;

    if base == overlay {
        return MergeOutcome::Scopes(vec![base]);
    }
    let incompatible = || CompositionConflictDetail::ActivationIncompatible {
        target: target.display(),
        base: base.display(),
        overlay: overlay.display(),
    };
    match (&base, &overlay) {
        (Always, _) | (_, Always) => match strategy.activation_merge {
            RequireCompatible => MergeOutcome::Conflict(incompatible()),
            Union => MergeOutcome::Scopes(vec![Always]),
        },
        (ActivationScope::WhenContextNote(_), ActivationScope::WhenContextNote(_)) => {
            match strategy.activation_merge {
                RequireCompatible => MergeOutcome::Conflict(incompatible()),
                // OR semantics: the target is active when either condition
                // holds; both rules are recorded.
                Union => MergeOutcome::Scopes(vec![base, overlay]),
            }
        }
    }
}

/// Typed composition-conflict detail.
///
/// Every unresolved semantic conflict surfaces as exactly one of these;
/// nothing is silently resolved.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CompositionConflictDetail {
    /// Both parents pin the same declared dependency to different
    /// immutable identities.
    #[error(
        "dependency `{key}` is re-pinned: base resolves to {base}, overlay resolves to {overlay}"
    )]
    DependencyRePin {
        /// The declared dependency's stable key.
        key: String,
        /// The base parent's resolved identity.
        base: String,
        /// The overlay parent's resolved identity.
        overlay: String,
    },
    /// A policy identity maps to different content in the two parents, or a
    /// policy reference validates the same identity against different
    /// content.
    #[error("policy {policy_id} has conflicting content between the parents")]
    PolicyContentMismatch {
        /// The contested policy identity.
        policy_id: String,
    },
    /// The parents activate the same target incompatibly.
    #[error("activation for `{target}` is incompatible: base is {base}, overlay is {overlay}")]
    ActivationIncompatible {
        /// The contested target.
        target: String,
        /// The base parent's scope.
        base: String,
        /// The overlay parent's scope.
        overlay: String,
    },
    /// An activation rule targets an element absent from the merged state.
    #[error("activation target `{target}` is not part of the merged system state")]
    ActivationTargetMissing {
        /// The dangling target.
        target: String,
    },
    /// A conditional activation's context note is absent from the composed
    /// mission's context model.
    #[error("activation context note `{note}` is not part of the composed mission context")]
    DanglingActivationContext {
        /// The dangling note.
        note: String,
    },
    /// The parents' assurance policies cannot compose (for example
    /// different model pins).
    #[error("assurance policies are incompatible: {reason}")]
    AssuranceIncompatible {
        /// The underlying assurance composition failure.
        reason: String,
    },
    /// An orthogonal composition requires a composed mission header.
    #[error(
        "orthogonal composition requires a composed mission header (mission authority is explicit)"
    )]
    MissionHeaderRequired,
    /// A specialization was given a mission header, but the overlay's
    /// mission governs verbatim.
    #[error("specialization does not take a composed mission header; the overlay mission governs")]
    MissionHeaderUnexpected,
    /// A composition record carried the exact same activation rule twice.
    #[error("duplicate activation rule for `{target}`")]
    DuplicateActivation {
        /// The duplicated target.
        target: String,
    },
}

#[cfg(test)]
#[path = "composition_tests.rs"]
mod tests;
