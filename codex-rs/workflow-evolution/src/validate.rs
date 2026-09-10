//! Candidate validation: replay, differential comparison, policy checks.
//!
//! The pipeline is the gate between a proposal and anything the governed
//! evolution plane is willing to publish. It is composed of ports:
//!
//! - [`ReplayPort`] — replays the incumbent and the sealed candidate
//!   version through a deterministic evaluation harness. The in-memory
//!   [`EvalReplayPort`] drives eval-compat's `WorkflowEvalHarness` with
//!   fully scripted model and environment doubles.
//! - the **differential gate** — eval-compat's `compare_workflow_runs`
//!   over the two replay records: any enumerated divergence fails the
//!   gate. A candidate that changes observable behavior is never
//!   promoted through this plane.
//! - [`PolicyPort`] — authorization (is this change category allowed?),
//!   resource (may bindings introduce capabilities the incumbent never
//!   declared?), and compatibility (is the successor version a legal
//!   bump?) checks.
//!
//! Every stage records evidence ([`StageEvidence`]); a candidate's report
//! passes only when all three stages ran and passed **explicitly**. A
//! failed replay fails the differential stage too (a skipped gate is a
//! failed gate); no stage ever fails silently.

use std::collections::BTreeSet;
use std::sync::Arc;

use codex_eval_compat::EVAL_MODEL_SLUG;
use codex_eval_compat::ScriptedEnvTurn;
use codex_eval_compat::ScriptedModelProvider;
use codex_eval_compat::ScriptedTurn;
use codex_eval_compat::WorkflowDivergence;
use codex_eval_compat::WorkflowEvalHarness;
use codex_eval_compat::WorkflowRunRecord;
use codex_eval_compat::compare_workflow_runs;
use codex_eval_compat::equivalent_capabilities;
use codex_workflow_app::WalkConfig;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_forge::PublishedVersionRef;
use serde::Deserialize;
use serde::Serialize;

use crate::CandidateId;
use crate::ChangeKind;
use crate::ImprovementCandidate;
use crate::ProposedChange;
use crate::VersionBump;
use crate::WorkflowEvolutionError;

/// The provider identity the replay harness registers its scripted model
/// double under.
pub const REPLAY_PROVIDER_ID: &str = "evolution-replay-provider";

/// The outcome of one replay: the two records the differential gate
/// compares.
#[derive(Clone, Debug)]
pub struct ReplayOutcome {
    /// The incumbent's replay record.
    pub baseline: WorkflowRunRecord,
    /// The candidate's replay record.
    pub candidate: WorkflowRunRecord,
}

/// The replay seam: re-executes the incumbent and the candidate version
/// under a deterministic evaluation harness.
///
/// Implementations are evaluation runtimes, not record seams; they are
/// generic parameters of the pipeline (static dispatch) so the trait can
/// expose a native async contract without object-safety workarounds. The
/// host's durable harness implements this the same way the in-memory
/// [`EvalReplayPort`] does.
pub trait ReplayPort: Send + Sync {
    /// Replays `incumbent` and `candidate` with identical, deterministic
    /// scripts and returns both run records.
    fn replay(
        &self,
        incumbent: &WorkflowVersion,
        candidate: &WorkflowVersion,
    ) -> impl Future<Output = Result<ReplayOutcome, WorkflowEvolutionError>> + Send;
}

/// The in-memory replay port over eval-compat's deterministic harness.
///
/// One port serves any number of replays: every replay builds two fresh
/// harnesses (one per side) from cloned scripts, so replays never consume
/// each other's state and identical inputs replay identically.
#[derive(Clone, Debug)]
pub struct EvalReplayPort {
    model_script: Vec<ScriptedTurn>,
    env_script: Vec<ScriptedEnvTurn>,
}

impl EvalReplayPort {
    /// Creates the port serving `model_script` (action proposals) and
    /// `env_script` (environment turns) to every replay.
    pub fn new(model_script: Vec<ScriptedTurn>, env_script: Vec<ScriptedEnvTurn>) -> Self {
        Self {
            model_script,
            env_script,
        }
    }

    /// Builds one fresh harness with cloned scripts.
    fn harness(&self) -> Result<WorkflowEvalHarness, WorkflowEvolutionError> {
        let provider = Arc::new(
            ScriptedModelProvider::new(REPLAY_PROVIDER_ID, self.model_script.clone())
                .with_capability(EVAL_MODEL_SLUG, equivalent_capabilities(REPLAY_PROVIDER_ID)),
        );
        Ok(WorkflowEvalHarness::new(provider, self.env_script.clone())?)
    }
}

impl ReplayPort for EvalReplayPort {
    async fn replay(
        &self,
        incumbent: &WorkflowVersion,
        candidate: &WorkflowVersion,
    ) -> Result<ReplayOutcome, WorkflowEvolutionError> {
        let mut baseline_harness = self.harness()?;
        let baseline_version = baseline_harness.publish_version_record(incumbent.clone())?;
        let baseline = baseline_harness
            .run(&baseline_version, WalkConfig::default())
            .await?;
        let mut candidate_harness = self.harness()?;
        let candidate_version = candidate_harness.publish_version_record(candidate.clone())?;
        let candidate = candidate_harness
            .run(&candidate_version, WalkConfig::default())
            .await?;
        Ok(ReplayOutcome {
            baseline,
            candidate,
        })
    }
}

/// One named policy check inside the policy gate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PolicyCheck {
    /// The check's stable name (`authorization`, `resource`, or
    /// `compatibility`).
    pub name: &'static str,
    /// Whether the check passed.
    pub passed: bool,
    /// The check's outcome, for reviewers.
    pub detail: String,
}

/// The inputs of one policy evaluation.
#[derive(Clone, Debug)]
pub struct PolicyContext<'a> {
    /// The candidate being validated.
    pub candidate: &'a ImprovementCandidate,
    /// The incumbent version record the candidate evolves.
    pub incumbent: &'a WorkflowVersion,
    /// The proposed successor semantic version.
    pub successor: &'a SemanticVersion,
}

/// The policy seam: authorization, resource, and compatibility checks.
///
/// The port is a pure decision seam (synchronous by design): hosts wire
/// it to their control-plane policy. The default in-memory implementation
/// is [`PolicyGate`] over an [`EvolutionPolicy`].
pub trait PolicyPort: Send + Sync {
    /// Evaluates every policy check for `context`; the gate passes only
    /// when every returned check passes.
    fn checks(&self, context: &PolicyContext<'_>) -> Vec<PolicyCheck>;
}

/// The data policy of the default [`PolicyGate`].
#[derive(Clone, Debug)]
pub struct EvolutionPolicy {
    /// Which change categories may be promoted at all.
    pub allowed_changes: BTreeSet<ChangeKind>,
    /// Which successor version bumps are compatible.
    pub allowed_bumps: BTreeSet<VersionBump>,
    /// Whether binding changes may introduce capabilities the incumbent
    /// never declared. Off by default: capability escalation needs
    /// evidence, not a proposal.
    pub allow_capability_introduction: bool,
}

impl EvolutionPolicy {
    /// The conservative default: every category, patch and minor bumps,
    /// no capability introduction.
    pub fn conservative() -> Self {
        Self {
            allowed_changes: BTreeSet::from([
                ChangeKind::DefinitionDelta,
                ChangeKind::CapabilityBinding,
                ChangeKind::RecoveryPolicy,
                ChangeKind::DependencyChoice,
                ChangeKind::ScheduleTuning,
            ]),
            allowed_bumps: BTreeSet::from([VersionBump::Patch, VersionBump::Minor]),
            allow_capability_introduction: false,
        }
    }
}

impl Default for EvolutionPolicy {
    fn default() -> Self {
        Self::conservative()
    }
}

/// The default in-memory policy gate over an [`EvolutionPolicy`].
#[derive(Clone, Debug)]
pub struct PolicyGate {
    policy: EvolutionPolicy,
}

impl PolicyGate {
    /// Creates the gate enforcing `policy`.
    pub fn new(policy: EvolutionPolicy) -> Self {
        Self { policy }
    }

    /// The authorization check: the change category must be allowed.
    fn authorization(&self, context: &PolicyContext<'_>) -> PolicyCheck {
        let kind = context.candidate.change_kind();
        let passed = self.policy.allowed_changes.contains(&kind);
        PolicyCheck {
            name: "authorization",
            passed,
            detail: format!(
                "change category `{}` is {} by policy",
                kind.key(),
                if passed { "allowed" } else { "refused" }
            ),
        }
    }

    /// The resource check: binding changes must not introduce capabilities
    /// the incumbent definition never declared.
    fn resource(&self, context: &PolicyContext<'_>) -> PolicyCheck {
        let declared: BTreeSet<String> = context
            .incumbent
            .definition
            .ir
            .nodes
            .values()
            .filter_map(|node| match node {
                codex_workflow_contracts::WorkflowIrNode::Step(step) => Some(&step.capabilities),
                _ => None,
            })
            .flatten()
            .map(|requirement| requirement.capability.as_ref().to_string())
            .collect();
        let introduced: BTreeSet<String> = match &context.candidate.change {
            ProposedChange::Bindings { bindings } => bindings
                .values()
                .flatten()
                .map(|requirement| requirement.capability.as_ref().to_string())
                .filter(|capability| !declared.contains(capability))
                .collect(),
            _ => BTreeSet::new(),
        };
        let passed = introduced.is_empty()
            || (self.policy.allow_capability_introduction && !introduced.is_empty());
        PolicyCheck {
            name: "resource",
            passed,
            detail: if introduced.is_empty() {
                "binding change stays within the incumbent's declared capabilities".to_string()
            } else {
                format!(
                    "binding change introduces undeclared capabilit(ies): {}",
                    introduced
                        .iter()
                        .map(|capability| format!("`{capability}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            },
        }
    }

    /// The compatibility check: the successor version must be exactly one
    /// allowed bump of the incumbent's version.
    fn compatibility(&self, context: &PolicyContext<'_>) -> PolicyCheck {
        let incumbent = &context.incumbent.identity.semantic_version;
        let bumps: Vec<&VersionBump> = self
            .policy
            .allowed_bumps
            .iter()
            .filter(|bump| bump.apply(incumbent) == *context.successor)
            .collect();
        PolicyCheck {
            name: "compatibility",
            passed: !bumps.is_empty(),
            detail: if bumps.is_empty() {
                format!(
                    "successor `{}` is not one of the policy's allowed bumps of incumbent \
                     `{incumbent}`",
                    context.successor
                )
            } else {
                format!(
                    "successor `{}` is an allowed {} bump of incumbent `{incumbent}`",
                    context.successor,
                    bumps[0].key()
                )
            },
        }
    }
}

impl VersionBump {
    /// The stable key of this bump direction.
    pub fn key(self) -> &'static str {
        match self {
            Self::Patch => "patch",
            Self::Minor => "minor",
            Self::Major => "major",
        }
    }
}

impl PolicyPort for PolicyGate {
    fn checks(&self, context: &PolicyContext<'_>) -> Vec<PolicyCheck> {
        vec![
            self.authorization(context),
            self.resource(context),
            self.compatibility(context),
        ]
    }
}

/// The name of one validation stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StageName {
    /// The replay stage.
    Replay,
    /// The differential-comparison stage.
    Differential,
    /// The policy stage.
    Policy,
}

impl StageName {
    /// The stable key of this stage.
    pub fn key(self) -> &'static str {
        match self {
            Self::Replay => "replay",
            Self::Differential => "differential",
            Self::Policy => "policy",
        }
    }
}

/// The serializable summary of one stage outcome (used in lineage).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StageSummary {
    /// The stage.
    pub stage: StageName,
    /// Whether the stage passed.
    pub passed: bool,
}

/// The evidence recorded by one validation stage.
#[derive(Clone, Debug)]
pub enum StageEvidence {
    /// The replay stage: both fingerprints, and the failure that prevented
    /// a replay, when any.
    Replay {
        /// The incumbent replay's semantic fingerprint.
        baseline_fingerprint: Option<ContentDigest>,
        /// The candidate replay's semantic fingerprint.
        candidate_fingerprint: Option<ContentDigest>,
        /// Why the replay failed, when it failed.
        failure: Option<String>,
        /// Whether the stage passed.
        passed: bool,
    },
    /// The differential stage: every enumerated divergence, and why the
    /// comparison was skipped, when it was.
    Differential {
        /// The enumerated divergences between the two replay records.
        divergences: Vec<WorkflowDivergence>,
        /// Why the comparison could not run (a failed replay), when any.
        skipped_reason: Option<String>,
        /// Whether the stage passed.
        passed: bool,
    },
    /// The policy stage: every check, in evaluation order.
    Policy {
        /// The policy checks that ran.
        checks: Vec<PolicyCheck>,
        /// Whether the stage passed.
        passed: bool,
    },
}

impl StageEvidence {
    /// The stage this evidence belongs to.
    pub fn stage(&self) -> StageName {
        match self {
            Self::Replay { .. } => StageName::Replay,
            Self::Differential { .. } => StageName::Differential,
            Self::Policy { .. } => StageName::Policy,
        }
    }

    /// Whether this stage passed explicitly.
    pub fn passed(&self) -> bool {
        match self {
            Self::Replay { passed, .. } => *passed,
            Self::Differential { passed, .. } => *passed,
            Self::Policy { passed, .. } => *passed,
        }
    }

    /// The serializable summary of this stage.
    pub fn summary(&self) -> StageSummary {
        StageSummary {
            stage: self.stage(),
            passed: self.passed(),
        }
    }
}

/// The full validation report of one candidate.
///
/// The report passes only when it carries exactly the three pipeline
/// stages and every one of them passed.
#[derive(Clone, Debug)]
pub struct ValidationReport {
    /// The validated candidate.
    pub candidate: CandidateId,
    /// The stages, in pipeline order.
    pub stages: Vec<StageEvidence>,
}

impl ValidationReport {
    /// Whether every gate passed explicitly.
    pub fn passed(&self) -> bool {
        self.stages.len() == 3
            && self.stages.iter().map(StageEvidence::stage).eq([
                StageName::Replay,
                StageName::Differential,
                StageName::Policy,
            ])
            && self.stages.iter().all(StageEvidence::passed)
    }

    /// The serializable stage summaries (used in lineage).
    pub fn summaries(&self) -> Vec<StageSummary> {
        self.stages.iter().map(StageEvidence::summary).collect()
    }

    /// The content digest binding this report to its outcome: the
    /// candidate, the successor version, the replay fingerprints, the
    /// divergence count, and the stage results.
    ///
    /// Approvals reference this digest, so an approval for one validation
    /// never transfers to a different one.
    pub fn digest(
        &self,
        successor: &PublishedVersionRef,
    ) -> Result<ContentDigest, WorkflowEvolutionError> {
        let projection = serde_json::json!({
            "candidate": self.candidate.to_string(),
            "successor": successor.version_id.to_string(),
            "stages": self
                .stages
                .iter()
                .map(StageEvidence::summary_projection)
                .collect::<Vec<_>>(),
        });
        Ok(ContentDigest::of(&projection)?)
    }

    /// The stages that failed, by key (for error reporting).
    pub fn failed_stages(&self) -> Vec<&'static str> {
        self.stages
            .iter()
            .filter(|stage| !stage.passed())
            .map(|stage| stage.stage().key())
            .collect()
    }
}

impl StageEvidence {
    /// The digest projection of this stage: its name, result, and, where
    /// present, its fingerprints and divergence count.
    fn summary_projection(&self) -> serde_json::Value {
        match self {
            Self::Replay {
                baseline_fingerprint,
                candidate_fingerprint,
                failure,
                passed,
            } => serde_json::json!({
                "stage": StageName::Replay.key(),
                "passed": passed,
                "baselineFingerprint": baseline_fingerprint.as_ref().map(ContentDigest::as_str),
                "candidateFingerprint": candidate_fingerprint.as_ref().map(ContentDigest::as_str),
                "failure": failure,
            }),
            Self::Differential {
                divergences,
                skipped_reason,
                passed,
            } => serde_json::json!({
                "stage": StageName::Differential.key(),
                "passed": passed,
                "divergences": divergences.len(),
                "skippedReason": skipped_reason,
            }),
            Self::Policy { checks, passed } => serde_json::json!({
                "stage": StageName::Policy.key(),
                "passed": passed,
                "checks": checks
                    .iter()
                    .map(|check| serde_json::json!({
                        "name": check.name,
                        "passed": check.passed,
                    }))
                    .collect::<Vec<_>>(),
            }),
        }
    }
}

/// The candidate validation pipeline.
///
/// Generic over the replay runtime and the policy gate; the differential
/// comparison is the pipeline's own fixed gate (eval-compat's
/// `compare_workflow_runs`).
pub struct ValidationPipeline<R: ReplayPort, P: PolicyPort> {
    replay: R,
    policy: P,
}

impl<R: ReplayPort, P: PolicyPort> ValidationPipeline<R, P> {
    /// Creates the pipeline over its ports.
    pub fn new(replay: R, policy: P) -> Self {
        Self { replay, policy }
    }

    /// Validates one candidate's sealed successor version against its
    /// incumbent.
    ///
    /// Stages, in order:
    ///
    /// 1. **replay** — both versions replay through the replay port; a
    ///    harness failure fails the stage (loudly, with the failure).
    /// 2. **differential** — `compare_workflow_runs` over the two
    ///    records; any enumerated divergence fails the gate. A failed
    ///    replay skips — and therefore fails — this stage.
    /// 3. **policy** — every policy check must pass.
    pub async fn validate(
        &self,
        candidate: &ImprovementCandidate,
        incumbent: &WorkflowVersion,
        successor_version: &WorkflowVersion,
    ) -> Result<ValidationReport, WorkflowEvolutionError> {
        let (replay_stage, differential_stage) =
            match self.replay.replay(incumbent, successor_version).await {
                Ok(outcome) => {
                    let baseline_fingerprint = outcome.baseline.fingerprint_digest().ok();
                    let candidate_fingerprint = outcome.candidate.fingerprint_digest().ok();
                    let replay_passed =
                        baseline_fingerprint.is_some() && candidate_fingerprint.is_some();
                    let replay_stage = StageEvidence::Replay {
                        baseline_fingerprint,
                        candidate_fingerprint,
                        failure: None,
                        passed: replay_passed,
                    };
                    let differential_stage = if replay_passed {
                        let differential =
                            compare_workflow_runs(&outcome.baseline, &outcome.candidate);
                        StageEvidence::Differential {
                            divergences: differential.divergences,
                            skipped_reason: None,
                            passed: differential.equivalent,
                        }
                    } else {
                        StageEvidence::Differential {
                            divergences: Vec::new(),
                            skipped_reason: Some(
                                "replay did not produce two complete records".to_string(),
                            ),
                            passed: false,
                        }
                    };
                    (replay_stage, differential_stage)
                }
                Err(error) => {
                    let failure = format!("replay of candidate `{}` failed: {error}", candidate.id);
                    (
                        StageEvidence::Replay {
                            baseline_fingerprint: None,
                            candidate_fingerprint: None,
                            failure: Some(failure.clone()),
                            passed: false,
                        },
                        StageEvidence::Differential {
                            divergences: Vec::new(),
                            skipped_reason: Some(failure),
                            passed: false,
                        },
                    )
                }
            };
        let policy_stage = self.policy_stage(candidate, incumbent, successor_version);
        Ok(ValidationReport {
            candidate: candidate.id.clone(),
            stages: vec![replay_stage, differential_stage, policy_stage],
        })
    }

    /// Runs and records the policy stage.
    fn policy_stage(
        &self,
        candidate: &ImprovementCandidate,
        incumbent: &WorkflowVersion,
        successor_version: &WorkflowVersion,
    ) -> StageEvidence {
        let context = PolicyContext {
            candidate,
            incumbent,
            successor: &successor_version.identity.semantic_version,
        };
        let checks = self.policy.checks(&context);
        let passed = checks.iter().all(|check| check.passed);
        StageEvidence::Policy { checks, passed }
    }
}
