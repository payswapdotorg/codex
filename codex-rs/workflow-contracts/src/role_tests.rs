use super::*;
use pretty_assertions::assert_eq;

fn reviewer() -> WorkflowRole {
    WorkflowRole {
        role: RoleId::parse("release_reviewer").expect("valid role id"),
        name: "Release reviewer".to_owned(),
        objective: "Approve the release before publication.".to_owned(),
        allowed_capabilities: vec![
            CapabilityId::parse("read_release_notes").expect("valid capability"),
        ],
        approval: ApprovalRequirement::Required,
    }
}

#[test]
fn roles_round_trip_through_serde() {
    let role = reviewer();

    let serialized = serde_json::to_string(&role).expect("role serializes");
    let parsed: WorkflowRole = serde_json::from_str(&serialized).expect("role deserializes");

    assert_eq!(role, parsed);
}

#[test]
fn roles_require_approval_explicitly() {
    let role = reviewer();
    assert_eq!(role.approval, ApprovalRequirement::Required);

    let unprivileged = WorkflowRole {
        approval: ApprovalRequirement::NotRequired,
        ..reviewer()
    };
    assert_eq!(unprivileged.approval, ApprovalRequirement::NotRequired);
}

#[test]
fn role_ids_reject_empty_values() {
    let result = RoleId::parse("");
    assert!(result.is_err(), "empty role ids must be rejected");
}

#[test]
fn approval_requirement_serializes_as_camel_case() {
    let serialized =
        serde_json::to_string(&ApprovalRequirement::Required).expect("approval serializes");
    assert_eq!(serialized, "\"required\"");
}
