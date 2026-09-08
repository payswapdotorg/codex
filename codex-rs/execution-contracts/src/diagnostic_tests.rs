use super::*;
use pretty_assertions::assert_eq;

#[test]
fn diagnostics_carry_code_message_and_environment() {
    let diagnostic =
        BindingDiagnostic::new(DiagnosticCode::NotInstalled, "browser runtime missing")
            .in_environment(ExecutionEnvironment::Browser);
    assert_eq!(diagnostic.code, DiagnosticCode::NotInstalled);
    assert_eq!(diagnostic.environment, Some(ExecutionEnvironment::Browser));
    assert_eq!(diagnostic.message, "browser runtime missing");
}

#[test]
fn oversized_messages_truncate_at_a_character_boundary() {
    let message = "é".repeat(3000);
    let diagnostic = BindingDiagnostic::new(DiagnosticCode::ProbeFailed, message);
    assert!(
        diagnostic.message.len() <= BindingDiagnostic::MAX_MESSAGE_BYTES + "...[truncated]".len()
    );
    assert!(diagnostic.message.ends_with("...[truncated]"));
    assert!(
        diagnostic
            .message
            .is_char_boundary(diagnostic.message.len())
    );
}

#[test]
fn diagnostics_round_trip_through_serde() {
    let diagnostic = BindingDiagnostic::new(
        DiagnosticCode::ResourceMissing,
        "browser_profile is not bound",
    )
    .in_environment(ExecutionEnvironment::Browser);
    let serialized = serde_json::to_string(&diagnostic).expect("serializable");
    let round_tripped: BindingDiagnostic =
        serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(round_tripped, diagnostic);
}

#[test]
fn every_diagnostic_code_round_trips() {
    for code in [
        DiagnosticCode::NotRegistered,
        DiagnosticCode::NotInstalled,
        DiagnosticCode::ProbeFailed,
        DiagnosticCode::PolicyDenied,
        DiagnosticCode::ApprovalDenied,
        DiagnosticCode::ResourceMissing,
        DiagnosticCode::ExecutionFailure,
        DiagnosticCode::EnvironmentLost,
        DiagnosticCode::ReservedEnvironment,
    ] {
        let serialized = serde_json::to_string(&code).expect("serializable");
        let round_tripped: DiagnosticCode =
            serde_json::from_str(&serialized).expect("deserializable");
        assert_eq!(round_tripped, code, "{serialized} must round-trip");
    }
}
