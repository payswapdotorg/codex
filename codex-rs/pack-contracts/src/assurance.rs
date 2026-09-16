//! Policy-scoped assurance and determinism contracts.
//!
//! Owned by the PACK-003 work order (Assurance / Determinism).
//!
//! Pack assurance uses policy-scoped dimensions rather than one global
//! determinism requirement. A pack or pack operation may independently
//! require:
//!
//! ```text
//! deterministic execution
//! replay
//! approval
//! evidence
//! model pinning
//! dependency pinning
//! environment pinning
//! ```
//!
//! Policies are composable, inspectable, and can report which dimensions
//! they require. The labels are policy presets, not semantic authority:
//! no global D0–D5 execution framework is hard-coded into runtime
//! semantics, and assurance policies must not leak into workflow semantics
//! or escalate authority.
//!
//! This module must not implement an experiment controller, automatic
//! rollback engine, or model routing; those belong to later Pack work
//! orders.
