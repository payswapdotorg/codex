//! Pack Policy contracts.
//!
//! Owned by the PACK-001 work order (Contract Foundation).
//!
//! A [`crate::PackPolicy`] record carries governed policy content whose
//! identity is a content-addressed [`crate::PackPolicyId`]. Policy authority
//! is explicit: policies constrain behavior inside the pack boundary, they
//! do not grant authority over workflow transitions, credentials, or
//! evidence.
//!
//! Assurance and determinism dimensions are policy-scoped and owned by the
//! `assurance` module (PACK-003); this module owns the general governing
//! policy records.
