//! Typed Pack contracts for the Codex Universal Pack / System-State platform.
//!
//! This crate defines Packs as governed, typed, immutable, provenance-bearing
//! control-plane abstractions that sit above Universal Workflows. It is a
//! contracts-only crate: it contains no agent runtime, no workflow engine, no
//! promotion controller, no Pack generator, no experiment controller, and no
//! durable storage. Those surfaces are owned by later Pack work orders.
//!
//! ## Authority model (frozen architecture 0.4.0)
//!
//! ```text
//! Pack
//!   != Agent Runtime
//!   != Workflow Engine
//!   != Authorization Authority
//!   != Credential Authority
//!   != Evidence Authority
//! ```
//!
//! The Workflow Control Plane remains the sole authority for legal durable
//! Workflow transitions. Pack state may reference immutable
//! [`codex_workflow_contracts::WorkflowVersionId`] identities, but it may not
//! redefine Workflow semantics. LLMs and Pack workers propose candidate
//! states; they do not own durable authority.
//!
//! ## Module map
//!
//! ```text
//! identity       ── PackId, PackRevisionId, PackPolicyId, PackProvenance
//! mission        ── Mission, value model, context model          (PACK-001)
//! constitution   ── Pack Constitution invariants                 (PACK-001)
//! policy         ── Pack Policy records                           (PACK-001)
//! dependency     ── Pack dependencies and dependency locks        (PACK-001)
//! revision       ── pack revision identity tuple, compatibility   (PACK-001)
//! system_state   ── PackSystemState, candidate/promoted revisions (PACK-002)
//! assurance      ── policy-scoped assurance / determinism         (PACK-003)
//! composition    ── two-parent pack composition contracts         (PACK-005)
//! ```
//!
//! ## Reused Universal contracts
//!
//! Pack contracts deliberately reuse the content-addressing and identity
//! machinery from [`codex_workflow_contracts`] (notably
//! [`codex_workflow_contracts::ContentDigest`]) instead of duplicating digest
//! or canonicalization semantics.
//!
//! ## Non-goals
//!
//! - No credential material ever enters a Pack artifact.
//! - No Flauz-specific or client-specific Pack semantics; Pack contracts are
//!   exposed through shared, versioned Universal contracts consumable by any
//!   future client (Desktop, Web, Mobile, Browser Extension, IDE, SDK).
//! - No global deterministic execution mode; assurance is policy-scoped and
//!   owned by the `assurance` module.

#![deny(missing_docs)]

mod assurance;
mod composition;
mod constitution;
mod dependency;
mod error;
mod identity;
mod mission;
mod policy;
mod revision;
mod system_state;

// The identity and error foundations are complete in the scaffold. The
// worker-owned modules (PACK-001: mission, constitution, policy, dependency,
// revision; PACK-002: system_state; PACK-003: assurance) add their re-exports
// below, keeping modules private and the public API explicitly exported.
pub use assurance::*;
pub use composition::*;
pub use constitution::*;
pub use dependency::*;
pub use error::*;
pub use identity::*;
pub use mission::*;
pub use policy::*;
pub use revision::*;
pub use system_state::*;
