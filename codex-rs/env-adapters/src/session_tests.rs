//! Tests for session ownership and takeover receipts.

use pretty_assertions::assert_eq;

use crate::session::BridgeSession;
use crate::session::OwnerToken;
use crate::session::TakeoverMode;
use crate::session::TakeoverReceipt;
use codex_execution_contracts::CapabilityBindingId;

#[test]
fn owner_tokens_compare_by_value() {
    assert_eq!(OwnerToken::new("a/b"), OwnerToken::new("a/b"));
    assert_ne!(OwnerToken::new("a/b"), OwnerToken::new("b/a"));
    assert_eq!(OwnerToken::new("a/b").as_str(), "a/b");
}

#[test]
fn sessions_are_owned_by_the_dispatch_identity() {
    let binding = CapabilityBindingId::parse("codex-remote-desktop-env:control_remote_desktop")
        .expect("binding id");
    let mut session = BridgeSession::new(binding.clone(), "ops-remote-host-1".to_string());
    assert_eq!(session.binding, binding);
    assert_eq!(session.target, "ops-remote-host-1");
    assert!(session.owned_by_dispatch());

    // A takeover displaces the dispatch owner until re-prepare.
    session.owner = OwnerToken::new("ops-human");
    assert!(!session.owned_by_dispatch());
}

#[test]
fn takeover_receipts_serialize_camel_case() {
    let binding = CapabilityBindingId::parse("codex-mobile-device-env:control_mobile_device")
        .expect("binding id");
    let receipt = TakeoverReceipt {
        binding: binding.clone(),
        target: "pixel-8-emulator".to_string(),
        previous_owner: OwnerToken::new("codex-mobile-device-env:control_mobile_device"),
        new_owner: OwnerToken::new("ops-human"),
        mode: TakeoverMode::Forced,
        reason: Some("operator intervention".to_string()),
    };
    let json = serde_json::to_value(&receipt).expect("serde");
    assert_eq!(json["binding"], binding.as_ref());
    assert_eq!(json["target"], "pixel-8-emulator");
    assert_eq!(json["mode"], "forced");
    assert_eq!(json["reason"], "operator intervention");
    assert_eq!(
        json["previousOwner"],
        "codex-mobile-device-env:control_mobile_device"
    );
}
