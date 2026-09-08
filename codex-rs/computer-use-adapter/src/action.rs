//! Normalized computer use actions and results.
//!
//! Actions are the adapter's normalized desktop-automation vocabulary. They
//! are workflow-agnostic: workflow semantics (nodes, roles, gates) live in
//! `codex-workflow-contracts` and are never encoded here. Sensitive payloads
//! are redacted in `Debug` and contribute only digests to fingerprints, so
//! credentials never reach logs or evidence.

use serde::Deserialize;
use serde::Serialize;
use std::fmt;

use crate::policy::ApplicationIdentity;
use crate::util::sha256_hex;

/// Mouse button selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MouseButton {
    /// Primary button.
    Left,
    /// Secondary button.
    Right,
    /// Middle button.
    Middle,
}

/// Keyboard modifier held during a key press.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyModifier {
    /// Control.
    Ctrl,
    /// Alt/Option.
    Alt,
    /// Shift.
    Shift,
    /// Command/Windows/Meta.
    Meta,
}

/// Pointer target in native screen coordinates, optionally scoped to a
/// window reference captured by a prior observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PointerTarget {
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
    /// Opaque window reference when the target is window-scoped.
    pub window_ref: Option<String>,
}

/// Credential-bearing text that must never reach logs or traces in clear.
#[derive(Clone, PartialEq, Eq)]
pub struct SensitiveText(String);

impl SensitiveText {
    /// Wraps sensitive text.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Exposes the plaintext to the bridge execution path only.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Digest of the plaintext; safe for logs, evidence, and fingerprints.
    pub fn fingerprint(&self) -> String {
        sha256_hex(self.0.as_bytes())
    }
}

impl fmt::Debug for SensitiveText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Credentials never enter logs: only the digest is shown.
        write!(f, "SensitiveText(sha256:{})", self.fingerprint())
    }
}

/// One normalized desktop action.
#[derive(Clone)]
pub enum NormalizedAction {
    /// Click at a pointer target.
    Click {
        /// Where to click.
        target: PointerTarget,
        /// Which button.
        button: MouseButton,
        /// Click count (1 single, 2 double).
        click_count: u8,
    },
    /// Type plain text; debug output shows the length only.
    TypeText {
        /// Text to type.
        text: String,
    },
    /// Type credential-bearing text; never rendered in clear anywhere.
    TypeSensitiveText {
        /// Secret text to type.
        text: SensitiveText,
    },
    /// Press a key with modifiers.
    PressKey {
        /// Key name (native surface vocabulary).
        key: String,
        /// Held modifiers.
        modifiers: Vec<KeyModifier>,
    },
    /// Scroll at a pointer target.
    Scroll {
        /// Where to scroll.
        target: PointerTarget,
        /// Vertical scroll delta.
        delta_y: i32,
    },
    /// Focus a window by reference from a prior observation.
    FocusWindow {
        /// Opaque window reference.
        window_ref: String,
    },
    /// Bring an application to the foreground / launch it.
    LaunchApplication {
        /// Application identity.
        application: ApplicationIdentity,
    },
    /// Capture a screenshot artifact.
    Screenshot,
    /// Wait before the next action.
    Wait {
        /// Milliseconds to wait.
        millis: u64,
    },
}

impl fmt::Debug for NormalizedAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Click {
                target,
                button,
                click_count,
            } => f
                .debug_struct("Click")
                .field("target", target)
                .field("button", button)
                .field("click_count", click_count)
                .finish(),
            Self::TypeText { text } => f
                .debug_struct("TypeText")
                .field("text", &format!("len={}", text.len()))
                .finish(),
            Self::TypeSensitiveText { .. } => f
                .debug_struct("TypeSensitiveText")
                .field("text", &"[redacted]")
                .finish(),
            Self::PressKey { key, modifiers } => f
                .debug_struct("PressKey")
                .field("key", key)
                .field("modifiers", modifiers)
                .finish(),
            Self::Scroll { target, delta_y } => f
                .debug_struct("Scroll")
                .field("target", target)
                .field("delta_y", delta_y)
                .finish(),
            Self::FocusWindow { window_ref } => f
                .debug_struct("FocusWindow")
                .field("window_ref", window_ref)
                .finish(),
            Self::LaunchApplication { application } => f
                .debug_struct("LaunchApplication")
                .field("application", application)
                .finish(),
            Self::Screenshot => f.debug_struct("Screenshot").finish(),
            Self::Wait { millis } => f.debug_struct("Wait").field("millis", millis).finish(),
        }
    }
}

impl NormalizedAction {
    /// Stable digest of the action; sensitive payloads contribute only their
    /// digest, never plaintext.
    pub fn fingerprint(&self) -> String {
        let description = match self {
            Self::Click {
                target,
                button,
                click_count,
            } => format!(
                "click|{}|{button:?}|{click_count}",
                target_fingerprint(target)
            ),
            Self::TypeText { text } => format!("type|{}", sha256_hex(text.as_bytes())),
            Self::TypeSensitiveText { text } => {
                format!("type-sensitive|{}", text.fingerprint())
            }
            Self::PressKey { key, modifiers } => format!("key|{key}|{modifiers:?}"),
            Self::Scroll { target, delta_y } => {
                format!("scroll|{}|{delta_y}", target_fingerprint(target))
            }
            Self::FocusWindow { window_ref } => format!("focus|{window_ref}"),
            Self::LaunchApplication { application } => {
                format!("launch|{}", application.digest())
            }
            Self::Screenshot => "screenshot".to_string(),
            Self::Wait { millis } => format!("wait|{millis}"),
        };
        sha256_hex(description.as_bytes())
    }

    /// Short human-readable summary safe for logs.
    pub fn summary(&self) -> String {
        match self {
            Self::Click {
                target,
                button,
                click_count,
            } => format!("click({target:?},{button:?},x{click_count})"),
            Self::TypeText { text } => format!("type(len={})", text.len()),
            Self::TypeSensitiveText { .. } => "type-sensitive[redacted]".to_string(),
            Self::PressKey { key, modifiers } => {
                if modifiers.is_empty() {
                    format!("press-key({key})")
                } else {
                    format!("press-key({key})+{modifiers:?}")
                }
            }
            Self::Scroll { target, delta_y } => format!("scroll({target:?},{delta_y})"),
            Self::FocusWindow { window_ref } => format!("focus({window_ref})"),
            Self::LaunchApplication { application } => {
                format!("launch({})", application.display_name)
            }
            Self::Screenshot => "screenshot".to_string(),
            Self::Wait { millis } => format!("wait({millis})"),
        }
    }

    /// The application identity this action directly targets, when
    /// expressible without a prior observation.
    pub fn target_application(&self) -> Option<&ApplicationIdentity> {
        match self {
            Self::LaunchApplication { application } => Some(application),
            _ => None,
        }
    }
}

fn target_fingerprint(target: &PointerTarget) -> String {
    format!(
        "{}|{}|{}",
        target.x,
        target.y,
        target.window_ref.as_deref().unwrap_or("-")
    )
}

/// Normalized failure reason for an executed action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActionFailureReason {
    /// Referenced window or target was not found.
    TargetNotFound,
    /// The native surface reported a permission problem.
    PermissionDenied,
    /// The native surface reported an error.
    #[serde(rename_all = "camelCase")]
    NativeError {
        /// Error text.
        message: String,
    },
    /// The action is unsupported by the native surface.
    Unsupported,
}

/// Terminal status of one executed action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActionStatus {
    /// The action completed successfully.
    Succeeded,
    /// The action executed and failed.
    #[serde(rename_all = "camelCase")]
    Failed {
        /// Why it failed.
        reason: ActionFailureReason,
    },
    /// The action was rejected before execution.
    #[serde(rename_all = "camelCase")]
    Rejected {
        /// Why it was rejected.
        reason: String,
    },
}

/// Normalized result of one executed action; normalized into workflow
/// evidence as an observation-kind record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NormalizedActionResult {
    /// Digest (fingerprint) of the executed action.
    pub action_fingerprint: String,
    /// Log-safe summary of the action.
    pub action_summary: String,
    /// Terminal status.
    pub status: ActionStatus,
    /// Unix milliseconds at completion (adapter clock).
    pub completed_at_unix_ms: u64,
}

impl NormalizedActionResult {
    /// Builds a succeeded result for `action`.
    pub fn succeeded(action: &NormalizedAction, completed_at_unix_ms: u64) -> Self {
        Self {
            action_fingerprint: action.fingerprint(),
            action_summary: action.summary(),
            status: ActionStatus::Succeeded,
            completed_at_unix_ms,
        }
    }

    /// Builds a failed result for `action`.
    pub fn failed(
        action: &NormalizedAction,
        reason: ActionFailureReason,
        completed_at_unix_ms: u64,
    ) -> Self {
        Self {
            action_fingerprint: action.fingerprint(),
            action_summary: action.summary(),
            status: ActionStatus::Failed { reason },
            completed_at_unix_ms,
        }
    }

    /// Builds a rejected result for `action`.
    pub fn rejected(
        action: &NormalizedAction,
        reason: impl Into<String>,
        completed_at_unix_ms: u64,
    ) -> Self {
        Self {
            action_fingerprint: action.fingerprint(),
            action_summary: action.summary(),
            status: ActionStatus::Rejected {
                reason: reason.into(),
            },
            completed_at_unix_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::KeyModifier;
    use super::MouseButton;
    use super::NormalizedAction;
    use super::NormalizedActionResult;
    use super::PointerTarget;
    use super::SensitiveText;
    use crate::policy::ApplicationIdentity;
    use crate::policy::ApplicationPlatformIdentity;

    fn app() -> ApplicationIdentity {
        ApplicationIdentity {
            display_name: "Editor".to_string(),
            platform: ApplicationPlatformIdentity::MacosBundle {
                bundle_id: "com.example.editor".to_string(),
            },
        }
    }

    #[test]
    fn sensitive_text_is_redacted_everywhere_but_expose() {
        let secret = SensitiveText::new("hunter2-password");
        assert_eq!(secret.expose(), "hunter2-password");
        let debug = format!("{secret:?}");
        assert!(!debug.contains("hunter2"), "debug redacts: {debug}");
        assert!(debug.contains("sha256:"));
    }

    #[test]
    fn typed_text_debug_shows_length_only() {
        let action = NormalizedAction::TypeText {
            text: "secret-in-plain".to_string(),
        };
        let debug = format!("{action:?}");
        assert!(!debug.contains("secret-in-plain"), "debug redacts: {debug}");
        assert!(debug.contains("len=15"));
    }

    #[test]
    fn fingerprints_are_stable_and_payload_sensitive() {
        let plain = NormalizedAction::TypeText {
            text: "abc".to_string(),
        };
        let plain_again = NormalizedAction::TypeText {
            text: "abc".to_string(),
        };
        let other = NormalizedAction::TypeText {
            text: "abd".to_string(),
        };
        assert_eq!(plain.fingerprint(), plain_again.fingerprint());
        assert_ne!(plain.fingerprint(), other.fingerprint());

        let secret = NormalizedAction::TypeSensitiveText {
            text: SensitiveText::new("hunter2"),
        };
        let same_secret = NormalizedAction::TypeSensitiveText {
            text: SensitiveText::new("hunter2"),
        };
        assert_eq!(secret.fingerprint(), same_secret.fingerprint());
        assert_ne!(secret.fingerprint(), plain.fingerprint());
        let summary = secret.summary();
        assert!(!summary.contains("hunter2"), "summary redacts: {summary}");
    }

    #[test]
    fn click_fingerprints_cover_target_and_button() {
        let left = NormalizedAction::Click {
            target: PointerTarget {
                x: 1,
                y: 2,
                window_ref: Some("w".to_string()),
            },
            button: MouseButton::Left,
            click_count: 1,
        };
        let right = NormalizedAction::Click {
            target: PointerTarget {
                x: 1,
                y: 2,
                window_ref: Some("w".to_string()),
            },
            button: MouseButton::Right,
            click_count: 1,
        };
        let double = NormalizedAction::Click {
            target: PointerTarget {
                x: 1,
                y: 2,
                window_ref: Some("w".to_string()),
            },
            button: MouseButton::Left,
            click_count: 2,
        };
        assert_ne!(left.fingerprint(), right.fingerprint());
        assert_ne!(left.fingerprint(), double.fingerprint());
    }

    #[test]
    fn results_serialize_camel_case_and_summarize() {
        let action = NormalizedAction::PressKey {
            key: "return".to_string(),
            modifiers: vec![KeyModifier::Meta],
        };
        let result = NormalizedActionResult::succeeded(&action, 3);
        assert_eq!(result.completed_at_unix_ms, 3);
        let json = serde_json::to_value(&result).expect("serde");
        assert_eq!(json["actionFingerprint"], result.action_fingerprint);
        assert_eq!(json["status"], "succeeded");
        assert!(result.action_summary.starts_with("press-key(return)"));
    }
}
