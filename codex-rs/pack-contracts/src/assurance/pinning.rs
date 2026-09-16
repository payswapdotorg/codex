//! The three pinning assurance dimensions.
//!
//! Model, dependency, and environment pinning declare that covered
//! execution must use a specific immutable identity, referenced by an
//! identifier and a content digest. Pin values are identity/digest
//! references only: they never carry credential material (tokens, keys, or
//! passwords), and the pinned content itself is owned by the respective
//! authority (the model registry, the dependency lock, the environment
//! descriptor store). A pin records what must be used, never who may use
//! it.
//!
//! Pins compose by equality: two policies pinning the same dimension to
//! the same identity compose cleanly, while any difference is an explicit
//! [`PackContractError::PolicyConflict`] because neither pin is stricter —
//! they are simply different immutable targets.

use codex_workflow_contracts::ContentDigest;
use serde::Deserialize;
use serde::Serialize;

use crate::PackContractError;

/// Maximum length of a pin identifier.
const PIN_IDENTIFIER_MAX_LENGTH: usize = 256;

/// A model pinned to immutable content.
///
/// The identifier names the model identity (for example
/// `atlas-reasoner-v2`) and the digest addresses the exact pinned model
/// content. The pin is a reference; model artifacts and model routing are
/// owned elsewhere.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelPin {
    model: String,
    digest: ContentDigest,
}

impl ModelPin {
    /// Creates a validated model pin.
    ///
    /// The model identifier must be non-empty, at most 256 characters, and
    /// contain no whitespace.
    pub fn new(model: impl Into<String>, digest: ContentDigest) -> Result<Self, PackContractError> {
        Ok(Self {
            model: validate_pin_identifier("model id", model.into())?,
            digest,
        })
    }

    /// The pinned model identity.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// The content digest of the pinned model.
    pub fn digest(&self) -> &ContentDigest {
        &self.digest
    }
}

/// A dependency pinned to immutable content.
///
/// The identifier names the dependency inside the pack's dependency
/// namespace (for example a workflow or capability identity) and the
/// digest addresses the exact pinned dependency content. The dependency
/// graph and its locks are owned by the PACK-001 dependency contracts;
/// this pin only declares the assurance requirement.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DependencyPin {
    dependency: String,
    digest: ContentDigest,
}

impl DependencyPin {
    /// Creates a validated dependency pin.
    ///
    /// The dependency identifier must be non-empty, at most 256
    /// characters, and contain no whitespace.
    pub fn new(
        dependency: impl Into<String>,
        digest: ContentDigest,
    ) -> Result<Self, PackContractError> {
        Ok(Self {
            dependency: validate_pin_identifier("dependency id", dependency.into())?,
            digest,
        })
    }

    /// The pinned dependency identity.
    pub fn dependency(&self) -> &str {
        &self.dependency
    }

    /// The content digest of the pinned dependency.
    pub fn digest(&self) -> &ContentDigest {
        &self.digest
    }
}

/// An execution environment pinned to immutable content.
///
/// The identifier names the environment and the digest addresses the exact
/// pinned environment descriptor. The descriptor bytes (image, toolchain,
/// configuration) are owned by the environment authority; the pin is a
/// reference, so no environment internals — and no credential material —
/// enter the policy.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnvironmentPin {
    environment: String,
    digest: ContentDigest,
}

impl EnvironmentPin {
    /// Creates a validated environment pin.
    ///
    /// The environment identifier must be non-empty, at most 256
    /// characters, and contain no whitespace.
    pub fn new(
        environment: impl Into<String>,
        digest: ContentDigest,
    ) -> Result<Self, PackContractError> {
        Ok(Self {
            environment: validate_pin_identifier("environment id", environment.into())?,
            digest,
        })
    }

    /// The pinned environment identity.
    pub fn environment(&self) -> &str {
        &self.environment
    }

    /// The content digest of the pinned environment descriptor.
    pub fn digest(&self) -> &ContentDigest {
        &self.digest
    }
}

/// The model pinning assurance dimension.
///
/// Declares whether covered execution must use a pinned model identity.
/// The requirement is declarative; model routing (choosing which model to
/// use when none is pinned) is not modeled here.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ModelPinningPolicy {
    /// A pinned model is not required.
    #[default]
    NotRequired,
    /// Covered execution must use the pinned model.
    Required {
        /// The pinned model identity and digest.
        pin: ModelPin,
    },
}

impl ModelPinningPolicy {
    /// Whether this dimension imposes no requirement.
    pub fn is_not_required(&self) -> bool {
        matches!(self, Self::NotRequired)
    }

    /// The enforced pin, or `None` when this dimension imposes no
    /// requirement.
    pub fn pin(&self) -> Option<&ModelPin> {
        match self {
            Self::NotRequired => None,
            Self::Required { pin } => Some(pin),
        }
    }

    /// Composes two model pinning requirements.
    ///
    /// Equal pins compose to the same pin; any difference is an explicit
    /// [`PackContractError::PolicyConflict`]. The operation is commutative
    /// — the conflict message names the two pins in a canonical order.
    pub fn compose(&self, other: &Self) -> Result<Self, PackContractError> {
        compose_pin(
            self.pin(),
            other.pin(),
            "model",
            ModelPin::model,
            ModelPin::digest,
        )
        .map(|pin| match pin {
            Some(pin) => Self::Required { pin },
            None => Self::NotRequired,
        })
    }
}

/// The dependency pinning assurance dimension.
///
/// Declares whether covered execution must use a pinned dependency
/// identity. The requirement is declarative; dependency resolution and
/// locking are owned by the PACK-001 dependency contracts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DependencyPinningPolicy {
    /// A pinned dependency is not required.
    #[default]
    NotRequired,
    /// Covered execution must use the pinned dependency.
    Required {
        /// The pinned dependency identity and digest.
        pin: DependencyPin,
    },
}

impl DependencyPinningPolicy {
    /// Whether this dimension imposes no requirement.
    pub fn is_not_required(&self) -> bool {
        matches!(self, Self::NotRequired)
    }

    /// The enforced pin, or `None` when this dimension imposes no
    /// requirement.
    pub fn pin(&self) -> Option<&DependencyPin> {
        match self {
            Self::NotRequired => None,
            Self::Required { pin } => Some(pin),
        }
    }

    /// Composes two dependency pinning requirements.
    ///
    /// Equal pins compose to the same pin; any difference is an explicit
    /// [`PackContractError::PolicyConflict`]. The operation is commutative
    /// — the conflict message names the two pins in a canonical order.
    pub fn compose(&self, other: &Self) -> Result<Self, PackContractError> {
        compose_pin(
            self.pin(),
            other.pin(),
            "dependency",
            DependencyPin::dependency,
            DependencyPin::digest,
        )
        .map(|pin| match pin {
            Some(pin) => Self::Required { pin },
            None => Self::NotRequired,
        })
    }
}

/// The environment pinning assurance dimension.
///
/// Declares whether covered execution must use a pinned execution
/// environment. The requirement is declarative; environment provisioning
/// and verification are owned elsewhere.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EnvironmentPinningPolicy {
    /// A pinned environment is not required.
    #[default]
    NotRequired,
    /// Covered execution must use the pinned environment.
    Required {
        /// The pinned environment identity and digest.
        pin: EnvironmentPin,
    },
}

impl EnvironmentPinningPolicy {
    /// Whether this dimension imposes no requirement.
    pub fn is_not_required(&self) -> bool {
        matches!(self, Self::NotRequired)
    }

    /// The enforced pin, or `None` when this dimension imposes no
    /// requirement.
    pub fn pin(&self) -> Option<&EnvironmentPin> {
        match self {
            Self::NotRequired => None,
            Self::Required { pin } => Some(pin),
        }
    }

    /// Composes two environment pinning requirements.
    ///
    /// Equal pins compose to the same pin; any difference is an explicit
    /// [`PackContractError::PolicyConflict`]. The operation is commutative
    /// — the conflict message names the two pins in a canonical order.
    pub fn compose(&self, other: &Self) -> Result<Self, PackContractError> {
        compose_pin(
            self.pin(),
            other.pin(),
            "environment",
            EnvironmentPin::environment,
            EnvironmentPin::digest,
        )
        .map(|pin| match pin {
            Some(pin) => Self::Required { pin },
            None => Self::NotRequired,
        })
    }
}

/// Shared composition kernel for the three pinning dimensions.
///
/// Returns the jointly required pin, or `None` when neither side requires
/// one; two different required pins are an explicit conflict.
fn compose_pin<T>(
    left: Option<&T>,
    right: Option<&T>,
    kind: &'static str,
    name: impl Fn(&T) -> &str,
    digest: impl Fn(&T) -> &ContentDigest,
) -> Result<Option<T>, PackContractError>
where
    T: Clone + PartialEq,
{
    match (left, right) {
        (None, None) => Ok(None),
        (Some(pin), None) | (None, Some(pin)) => Ok(Some(pin.clone())),
        (Some(left), Some(right)) if left == right => Ok(Some(left.clone())),
        (Some(left), Some(right)) => Err(PackContractError::PolicyConflict {
            reason: pin_conflict_reason(
                kind,
                (name(left), digest(left)),
                (name(right), digest(right)),
            ),
        }),
    }
}

/// Builds an order-independent conflict message for two different pins.
///
/// The two pins are named in a canonical (sorted) order so that composing
/// one policy with the other reports exactly the same conflict as
/// composing them the other way around.
fn pin_conflict_reason(
    kind: &'static str,
    left: (&str, &ContentDigest),
    right: (&str, &ContentDigest),
) -> String {
    let ((first_name, first_digest), (second_name, second_digest)) =
        if (left.0, left.1.as_str()) <= (right.0, right.1.as_str()) {
            (left, right)
        } else {
            (right, left)
        };
    format!(
        "{kind} pin conflict: `{first_name}` pinned at {first_digest} cannot compose with `{second_name}` pinned at {second_digest}"
    )
}

/// Validates a pin identifier: non-empty, at most 256 characters, and free
/// of whitespace.
fn validate_pin_identifier(kind: &'static str, value: String) -> Result<String, PackContractError> {
    let reason = if value.is_empty() {
        "must be non-empty"
    } else if value.len() > PIN_IDENTIFIER_MAX_LENGTH {
        "must be at most 256 characters"
    } else if value.trim().len() != value.len() {
        "must not start or end with whitespace"
    } else if value.chars().any(char::is_whitespace) {
        "must not contain whitespace"
    } else {
        return Ok(value);
    };
    Err(PackContractError::InvalidIdentifier {
        kind,
        value,
        reason,
    })
}
