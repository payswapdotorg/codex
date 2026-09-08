use super::*;
use pretty_assertions::assert_eq;

#[test]
fn adapter_ids_accept_codex_style_tokens() {
    for value in ["codex-browser-use", "terminal.local", "human_ops", "ext1"] {
        assert!(AdapterId::parse(value).is_ok(), "`{value}` must parse");
    }
}

#[test]
fn adapter_ids_reject_invalid_tokens() {
    for value in [
        "",
        ".hidden",
        "trail.",
        "double..dot",
        "sp ace",
        "slash/name",
    ] {
        assert!(
            AdapterId::parse(value).is_err(),
            "`{value}` must be rejected"
        );
    }
}

#[test]
fn binding_ids_derive_deterministically() {
    let adapter = AdapterId::parse("codex-browser-use").expect("valid adapter");
    let capability = CapabilityId::parse("navigate_web").expect("valid capability");
    let first = CapabilityBindingId::derive(&adapter, &capability);
    let second = CapabilityBindingId::derive(&adapter, &capability);
    assert_eq!(first, second);
    assert_eq!(first.to_string(), "codex-browser-use:navigate_web");
}

#[test]
fn binding_ids_parse_and_reject_malformed_forms() {
    assert!(CapabilityBindingId::parse("codex-browser-use:navigate_web").is_ok());
    for value in [
        "",
        "no-colon",
        ":missing-adapter",
        "missing-capability:",
        "two:colons:here",
    ] {
        assert!(
            CapabilityBindingId::parse(value).is_err(),
            "`{value}` must be rejected"
        );
    }
}

#[test]
fn binding_ids_round_trip_through_serde() {
    let binding = CapabilityBindingId::parse("terminal.local:run_terminal_command")
        .expect("valid binding id");
    let serialized = serde_json::to_string(&binding).expect("serializable");
    assert_eq!(serialized, "\"terminal.local:run_terminal_command\"");
    let parsed: CapabilityBindingId = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(parsed, binding);
}

#[test]
fn resource_ids_use_path_segment_rules() {
    assert!(ResourceId::parse("profile-default").is_ok());
    assert!(ResourceId::parse("github-acme").is_ok());
    assert!(ResourceId::parse("").is_err());
    assert!(ResourceId::parse("has space").is_err());
}

#[test]
fn session_handles_are_opaque_but_bounded() {
    assert!(SessionHandle::parse("session-1").is_ok());
    assert!(SessionHandle::parse("").is_err());
    assert!(SessionHandle::parse("line\nbreak").is_err());
    let oversized = "x".repeat(257);
    assert!(SessionHandle::parse(oversized).is_err());
}
