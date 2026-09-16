//! Pack system-state contracts.
//!
//! Owned by the PACK-002 work order (Pack System State).
//!
//! `PackSystemState` is the durable representation of the currently proposed
//! or promoted software system. Candidate state and promoted state are
//! structurally distinct: promotion creates a new immutable promoted
//! revision and never mutates the historical candidate.
//!
//! A pack references immutable
//! [`codex_workflow_contracts::WorkflowVersionId`] identities — not moving
//! branches or mutable workflow state. Workflow transitions remain owned by
//! the Workflow Control Plane.
//!
//! This module must not implement the promotion controller, rollback
//! engine, or pack composition; those belong to later Pack work orders.
