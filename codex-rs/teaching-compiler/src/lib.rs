//! Workflow teaching and compiler (WO-008).
//!
//! This crate turns teaching interaction — instruction, demonstration, or
//! a hybrid of both — into validated, publication-ready workflow
//! definitions built on the frozen semantic contracts
//! (`codex-workflow-contracts`).
//!
//! ## Scope and invariants
//!
//! - **No runtime.** The crate records teaching input, compiles it to a
//!   `WorkflowIr`-backed [`WorkflowCandidate`], validates and simulates
//!   the candidate, and gates publication behind explicit approval. It
//!   contains no agent runtime and no workflow engine; execution and
//!   durable instance lifecycle remain with their owning planes.
//! - **Teaching input is untrusted.** Trajectory records are bounded,
//!   structured, and validated. Transcripts and chat history are never
//!   the semantic authority: semantics exist only in the compiled IR, and
//!   raw text is carried as intent descriptions and provenance.
//! - **Environment-neutral semantics.** Steps name no concrete tools,
//!   models, browsers, or MCP servers. Capability, resource, dependency,
//!   and execution-binding outcomes are *proposals* for the execution
//!   plane; the compiler never writes bindings itself and never encodes
//!   provider-specific semantics.
//! - **Approval-gated, immutable publication.** A candidate becomes an
//!   [`ApprovedWorkflow`] only after validation, deterministic
//!   simulation, and an explicit approval bound to the exact candidate
//!   content epoch. The frozen content is proven byte-stable before
//!   hand-off and pinned with its canonical digest. Publishing the actual
//!   immutable workflow version is the workflow control plane's act; this
//!   crate never publishes or mutates versions itself.
//! - **Deterministic optimization, provably safe only.** Optimization is
//!   limited to transforms with structural semantic-equivalence proofs
//!   (unreachable-node removal and single-child-sequence collapse), gated
//!   on session policy, evidence for observed steps, and preservation of
//!   step content.
//!
//! ## Pipeline
//!
//! ```text
//! TeachingSession (bounded trajectory, one of three modes)
//!   -> compile()         WorkflowCandidate (IR + inferences)
//!   -> validate()        findings; error severity blocks
//!   -> simulate()        deterministic structural replay
//!   -> propose/apply optimization (optional, gated)
//!   -> approve()         approval bound to the content epoch
//!   -> finalize()        ApprovedWorkflow (definition + digest)
//!   -> control plane publishes the immutable workflow version
//! ```

#![deny(missing_docs)]

mod graph;

pub mod approval;
pub mod blueprint;
pub mod candidate;
pub mod compiler;
pub mod error;
pub mod evidence;
pub mod mode;
pub mod optimization;
pub mod proposal;
pub mod session;
pub mod simulation;
pub mod trajectory;
pub mod validation;

pub use approval::ApprovalDecision;
pub use approval::ApprovalDecisionKind;
pub use approval::ApprovalRecord;
pub use blueprint::StepBlueprint;
pub use blueprint::StepOrigin;
pub use candidate::ApprovedWorkflow;
pub use candidate::CandidateOrigin;
pub use candidate::CandidateStatus;
pub use candidate::WorkflowCandidate;
pub use compiler::compile;
pub use error::TeachingCompilerError;
pub use evidence::TeachingEvidence;
pub use mode::TeachingMode;
pub use optimization::OptimizationKind;
pub use optimization::OptimizationProposal;
pub use optimization::apply_optimization;
pub use optimization::propose_optimizations;
pub use session::DEFAULT_MAX_RECORDS;
pub use session::SessionPolicy;
pub use session::TeachingSession;
pub use simulation::SimulationConfig;
pub use simulation::SimulationOutcome;
pub use simulation::SimulationReport;
pub use simulation::simulate_ir;
pub use trajectory::RecordOrigin;
pub use trajectory::TrajectoryEvent;
pub use trajectory::TrajectoryRecord;
pub use validation::FindingCode;
pub use validation::Severity;
pub use validation::ValidationFinding;
pub use validation::ValidationSummary;
