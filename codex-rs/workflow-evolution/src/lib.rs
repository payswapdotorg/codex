//! Learning and governed evolution for the Codex universal workflow
//! platform (WO-014).
//!
//! This crate is the evidence-driven improvement and governed-evolution
//! plane: it turns **execution evidence** into traceable improvement
//! candidates, validates them through replay, differential comparison,
//! and policy checks, and — only under an explicit approval — publishes
//! them as new immutable workflow versions with complete lineage.
//!
//! # Position in the stack
//!
//! The crate is a leaf over the already-merged frozen surfaces:
//!
//! - `codex-workflow-contracts` (WO-003) owns versions, integrity,
//!   evidence references, and dependency locks;
//! - `codex-workflow-app` (WO-010) owns the application runtime and the
//!   evaluation harness's error channel;
//! - `codex-eval-compat` (WO-013) owns the deterministic evaluation
//!   harness and differential comparison this crate replays through;
//! - `codex-workflow-forge` (WO-009) owns version publication, releases,
//!   installation, and the reviewable update/rollback transitions;
//! - `codex-workflow-triggers` (WO-011) owns the schedule specifications
//!   schedule-tuning candidates adjust.
//!
//! Nothing here duplicates those surfaces: candidates are replayed
//! through eval-compat's harness, comparisons use eval-compat's
//! differential, publication and rollback use the forge's release and
//! install semantics, and immutability is the contract layer's integrity
//! check.
//!
//! # The governed lifecycle
//!
//! ```text
//! execution evidence (read-only corpus)
//!   -> CandidateGenerator (deterministic rules, full provenance)
//!   -> EvolutionLedger record (credential scrub + retention bounds)
//!   -> validate (mint revision, seal successor, replay, differential, policy)
//!   -> ApprovalPort decide (explicit human/policy decision only)
//!   -> publish successor (forge release + immutable lineage)
//!   -> upgrade / rollback installation (reviewable, recorded transitions)
//! ```
//!
//! # What this crate deliberately is not
//!
//! - **No second engine or runtime.** Validation replays through
//!   eval-compat's harness; every semantic decision belongs to the frozen
//!   crates.
//! - **No silent mutation.** The installed and published predecessor
//!   version is never mutated — publication only adds a successor, a
//!   release, and a lineage record. A candidate that would reproduce the
//!   incumbent content exactly is refused as a no-op.
//! - **No model-suggestion authority.** Model output may inspire a host
//!   to build a candidate, but only the approval port — fed by explicit
//!   human/policy principals — authorizes promotion. There is no API path
//!   from a model, run record, or generator to an approval.
//! - **No live engine state in learning.** Candidates are generated only
//!   from a closed, read-only [`EvidenceCorpus`].
//! - **No unbounded retention.** Candidates are capped by retention
//!   policy (FIFO eviction); provenance is reference-shaped (locators,
//!   digests, run fingerprints — never payloads); credential-shaped
//!   content is refused at every entry point (candidates, approvals,
//!   notes) by a dedicated scrubbing/checking step.
//! - **No credentials.** Repository and version identity stay canonical
//!   and credential-free; findings from the scrubber never echo the
//!   matched content.
//!
//! # Determinism rules
//!
//! - The generator is pure: the same corpus yields the same candidates,
//!   in the same order, with the same content-addressed ids.
//! - Replay uses fully scripted model and environment doubles through
//!   eval-compat's harnesses; identical inputs replay identically.
//! - Lineage references content digests, so governance records are
//!   auditable without embedding payloads.
//!
//! # Example shape
//!
//! ```text
//! install fixture -> run through harness -> evidence corpus
//!   -> generator -> candidate (provenance: refs + run fingerprints)
//!   -> validate: replay incumbent vs sealed successor -> equivalent
//!      -> differential: no divergences -> policy: all checks pass
//!   -> approve (explicit) -> publish release + lineage
//!   -> upgrade installation -> rollback via lineage (recorded)
//! ```

#![deny(missing_docs)]

mod approval;
mod candidate;
mod error;
mod evolution;
mod generate;
mod ledger;
mod scrub;
mod validate;

#[cfg(test)]
mod test_support;

pub use approval::{
    ApprovalDecision, ApprovalPort, ApprovalRecord, ApprovalRequest, InMemoryApprovalPort,
};
pub use candidate::{
    CandidateId, CandidateProvenance, ChangeKind, ImprovementCandidate, ProposedChange,
    RunProvenance, VersionBump,
};
pub use error::WorkflowEvolutionError;
pub use evolution::{
    AppliedAdjustment, EVOLUTION_AUTHOR, EvolutionGovernor, SuccessorPublication, VersionLineage,
};
pub use generate::{CandidateGenerator, EvidenceCorpus, GenerationPolicy};
pub use ledger::{
    CandidateOutcome, EvolutionLedger, LedgerEntry, RetentionPolicy, StagedSuccession,
};
pub use scrub::{REDACTED_MARKER, ScrubFinding, ScrubReport, require_clean_json, scrub_json};
pub use validate::{
    EvalReplayPort, EvolutionPolicy, PolicyCheck, PolicyContext, PolicyGate, PolicyPort,
    REPLAY_PROVIDER_ID, ReplayOutcome, ReplayPort, StageEvidence, StageName, StageSummary,
    ValidationPipeline, ValidationReport,
};
