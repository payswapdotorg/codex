//! Pack dependency contracts.
//!
//! Owned by the PACK-001 work order (Contract Foundation).
//!
//! Pack dependencies reference immutable identities: workflow version
//! identities ([`codex_workflow_contracts::WorkflowVersionId`]), capability
//! identities, and other pack revisions. A dependency update produces a
//! candidate pack revision; it never silently changes a promoted revision.
//!
//! Dependency locks are content-addressed so a
//! [`crate::PackRevisionId`] can pin an exact dependency state.
