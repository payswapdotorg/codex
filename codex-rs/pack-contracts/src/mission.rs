//! Mission, value model, and context model contracts for Packs.
//!
//! Owned by the PACK-001 work order (Contract Foundation).
//!
//! Every Pack has an explicit mission model that is user/organization
//! authority. The mission model distinguishes:
//!
//! ```text
//! Mission
//! Value Model
//! Context Model
//! Hard Constraints
//! User/Organization Preferences
//! Success Measures
//! ```
//!
//! An LLM-generated architecture is a proposal and may not silently redefine
//! the mission. Mission content is content-addressed so a [`crate::PackRevisionId`]
//! can pin an exact mission revision.
