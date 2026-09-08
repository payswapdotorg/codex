//! Normalized screen/application/window observations.
//!
//! Observations are untrusted external input: they are evidence, never
//! workflow authority, and they never carry browser origin semantics (browser
//! state is owned by the browser use adapter, WO-006). Window titles and
//! native window references are recorded as opaque, credential-free strings.

use serde::Deserialize;
use serde::Serialize;

use crate::policy::ApplicationIdentity;
use crate::util::canonical_digest;

/// Identity of a screen/monitor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenIdentity {
    /// Zero-based screen index reported by the native surface.
    pub index: u32,
    /// Optional screen name.
    pub name: Option<String>,
}

/// Window bounds in native screen coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowBounds {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

/// One observed window.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowObservation {
    /// Window title as reported by the native surface (untrusted input).
    pub title: String,
    /// Opaque native window reference (credential-free).
    pub window_ref: String,
    /// Whether the window has input focus.
    pub focused: bool,
    /// Window bounds when reported.
    pub bounds: Option<WindowBounds>,
}

/// One observed application with its windows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationObservation {
    /// Normalized application identity.
    pub application: ApplicationIdentity,
    /// Observed windows of this application.
    pub windows: Vec<WindowObservation>,
    /// Whether the application is foreground.
    pub foreground: bool,
}

/// A normalized screen/application/window observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenObservation {
    /// Screen the observation was captured on.
    pub screen: ScreenIdentity,
    /// Observed applications.
    pub applications: Vec<ApplicationObservation>,
    /// Unix milliseconds of capture (adapter clock).
    pub captured_at_unix_ms: u64,
}

impl ScreenObservation {
    /// Stable content digest over the canonical serialization; evidence
    /// records reference observations by this digest so drift is detectable.
    pub fn content_digest(&self) -> String {
        canonical_digest(self)
    }

    /// Resolves the application owning `window_ref`, when present in this
    /// observation.
    pub fn application_for_window(&self, window_ref: &str) -> Option<ApplicationIdentity> {
        self.applications
            .iter()
            .find(|application| {
                application
                    .windows
                    .iter()
                    .any(|window| window.window_ref == window_ref)
            })
            .map(|application| application.application.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::ApplicationObservation;
    use super::ScreenIdentity;
    use super::ScreenObservation;
    use super::WindowBounds;
    use super::WindowObservation;
    use crate::policy::ApplicationIdentity;
    use crate::policy::ApplicationPlatformIdentity;

    fn observation() -> ScreenObservation {
        ScreenObservation {
            screen: ScreenIdentity {
                index: 0,
                name: Some("main".to_string()),
            },
            applications: vec![ApplicationObservation {
                application: ApplicationIdentity {
                    display_name: "Editor".to_string(),
                    platform: ApplicationPlatformIdentity::MacosBundle {
                        bundle_id: "com.example.editor".to_string(),
                    },
                },
                windows: vec![WindowObservation {
                    title: "doc.txt".to_string(),
                    window_ref: "win-1".to_string(),
                    focused: true,
                    bounds: Some(WindowBounds {
                        x: 0,
                        y: 0,
                        width: 800,
                        height: 600,
                    }),
                }],
                foreground: true,
            }],
            captured_at_unix_ms: 10,
        }
    }

    #[test]
    fn observation_digest_is_stable_and_content_sensitive() {
        let first = observation().content_digest();
        let second = observation().content_digest();
        assert_eq!(first, second);
        let mut changed = observation();
        changed.applications[0].windows[0].focused = false;
        assert_ne!(first, changed.content_digest());
    }

    #[test]
    fn windows_resolve_to_application_identities() {
        let observation = observation();
        let resolved = observation
            .application_for_window("win-1")
            .expect("window resolves");
        assert_eq!(resolved.display_name, "Editor");
        assert!(observation.application_for_window("missing").is_none());
    }

    #[test]
    fn observations_carry_no_browser_semantics() {
        let serialized = serde_json::to_string(&observation()).expect("serde");
        assert!(!serialized.contains("origin"));
        assert!(!serialized.contains("url"));
    }
}
