//! Test doubles for the bridge boundary (compiled only under `cfg(test)`).

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;

use crate::action::ActionFailureReason;
use crate::action::NormalizedAction;
use crate::action::NormalizedActionResult;
use crate::bridge::BridgeCallFailure;
use crate::bridge::BridgeIdentity;
use crate::bridge::BridgeReadinessFailure;
use crate::bridge::ComputerUseBridge;
use crate::bridge::ComputerUseBridgeFactory;
use crate::observation::ScreenIdentity;
use crate::observation::ScreenObservation;
use crate::session::UnixMsClock;

/// Behavior script for a stub bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StubBridgeMode {
    /// Every action succeeds.
    Healthy,
    /// Every action reports a lost bridge.
    AlwaysLoses,
    /// The first action fails, later actions succeed.
    FailsFirstAction,
}

/// Stub bridge.
pub(crate) struct StubBridge {
    mode: StubBridgeMode,
    failures_left: Mutex<u32>,
}

impl StubBridge {
    pub(crate) fn new(mode: StubBridgeMode) -> Self {
        let failures_left = u32::from(mode == StubBridgeMode::FailsFirstAction);
        Self {
            mode,
            failures_left: Mutex::new(failures_left),
        }
    }

    fn stub_identity() -> BridgeIdentity {
        BridgeIdentity {
            bridge_kind: "stub-computer-use".to_string(),
            runtime: "node_repl".to_string(),
            version: "1.0.0-test".to_string(),
        }
    }
}

impl ComputerUseBridge for StubBridge {
    fn identity(&self) -> BridgeIdentity {
        Self::stub_identity()
    }

    fn probe(&self) -> Result<(), BridgeReadinessFailure> {
        Ok(())
    }

    fn execute(
        &self,
        action: NormalizedAction,
    ) -> Result<NormalizedActionResult, BridgeCallFailure> {
        match self.mode {
            StubBridgeMode::AlwaysLoses => Err(BridgeCallFailure::BridgeUnavailable {
                details: "stub bridge lost".to_string(),
            }),
            StubBridgeMode::Healthy => Ok(NormalizedActionResult::succeeded(&action, 0)),
            StubBridgeMode::FailsFirstAction => {
                let mut failures_left = self
                    .failures_left
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if *failures_left > 0 {
                    *failures_left -= 1;
                    Ok(NormalizedActionResult::failed(
                        &action,
                        ActionFailureReason::NativeError {
                            message: "stub failure".to_string(),
                        },
                        0,
                    ))
                } else {
                    Ok(NormalizedActionResult::succeeded(&action, 0))
                }
            }
        }
    }

    fn observe(&self) -> Result<ScreenObservation, BridgeCallFailure> {
        Ok(ScreenObservation {
            screen: ScreenIdentity {
                index: 0,
                name: Some("stub-screen".to_string()),
            },
            applications: Vec::new(),
            captured_at_unix_ms: 0,
        })
    }
}

/// Factory serving scripted bridge modes and counting provisions.
#[derive(Default)]
pub(crate) struct ScriptedFactory {
    modes: Mutex<VecDeque<StubBridgeMode>>,
    provisions: Mutex<u32>,
}

impl ScriptedFactory {
    /// Creates a factory that serves `modes` in order, then healthy bridges.
    pub(crate) fn scripted(modes: Vec<StubBridgeMode>) -> Self {
        Self {
            modes: Mutex::new(modes.into_iter().collect()),
            provisions: Mutex::new(0),
        }
    }

    /// Number of provision calls so far.
    pub(crate) fn provision_count(&self) -> u32 {
        *self
            .provisions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl ComputerUseBridgeFactory for ScriptedFactory {
    fn provision(&self) -> Result<Box<dyn ComputerUseBridge>, BridgeReadinessFailure> {
        *self
            .provisions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) += 1;
        let mode = self
            .modes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .pop_front()
            .unwrap_or(StubBridgeMode::Healthy);
        Ok(Box::new(StubBridge::new(mode)))
    }
}

/// Factory whose every provision fails with a fixed readiness failure.
pub(crate) struct UnprovisionedFactory {
    failure: BridgeReadinessFailure,
}

impl UnprovisionedFactory {
    pub(crate) fn new(failure: BridgeReadinessFailure) -> Self {
        Self { failure }
    }
}

impl ComputerUseBridgeFactory for UnprovisionedFactory {
    fn provision(&self) -> Result<Box<dyn ComputerUseBridge>, BridgeReadinessFailure> {
        Err(self.failure.clone())
    }
}

/// Deterministic clock advancing 1000 ms per call, starting at `start`.
pub(crate) fn ticking_clock(start: u64) -> UnixMsClock {
    let current = Mutex::new(start);
    Arc::new(move || {
        let mut value = current
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let now = *value;
        *value += 1_000;
        now
    })
}
