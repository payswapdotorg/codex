use super::*;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn teach_start_params_round_trip_with_optional_name() {
    let named = WorkflowTeachStartParams {
        mode: WorkflowTeachMode::Hybrid,
        name: Some("daily-standup-report".into()),
    };
    assert_eq!(
        serde_json::to_value(&named).unwrap(),
        json!({
            "mode": "hybrid",
            "name": "daily-standup-report"
        })
    );
    assert_eq!(
        serde_json::from_value::<WorkflowTeachStartParams>(serde_json::to_value(&named).unwrap())
            .unwrap(),
        named
    );
    let unnamed = WorkflowTeachStartParams {
        mode: WorkflowTeachMode::Demonstrate,
        name: None,
    };
    let decoded =
        serde_json::from_value::<WorkflowTeachStartParams>(json!({"mode": "demonstrate"})).unwrap();
    assert_eq!(decoded, unnamed);
}

#[test]
fn teach_record_params_round_trip_and_reject_unknown_fields() {
    let params = WorkflowTeachDemonstrateParams {
        session_id: "ws-1".into(),
        kind: WorkflowDemonstrationKind::Result,
        text: "the pane shows the summary".into(),
        evidence: vec![WorkflowTeachEvidenceInput {
            label: "open-details".into(),
            locator: "rollout://abc".into(),
            sha256: "ab".repeat(32),
        }],
    };
    assert_eq!(
        serde_json::to_value(&params).unwrap(),
        json!({
            "sessionId": "ws-1",
            "kind": "result",
            "text": "the pane shows the summary",
            "evidence": [{
                "label": "open-details",
                "locator": "rollout://abc",
                "sha256": "abababababababababababababababababababababababababababababababab"
            }]
        })
    );
    assert_eq!(
        serde_json::from_value::<WorkflowTeachDemonstrateParams>(
            serde_json::to_value(&params).unwrap()
        )
        .unwrap(),
        params
    );
    assert!(
        serde_json::from_value::<WorkflowTeachDemonstrateParams>(json!({
            "sessionId": "ws-1",
            "kind": "action",
            "text": "click",
            "unexpected": true
        }))
        .is_err()
    );
}

#[test]
fn publish_response_serializes_workflow_identity_fields() {
    let response = WorkflowPublishResponse {
        workflow: "daily-standup-report".into(),
        version_id: format!("sha256:{}", "cd".repeat(32)),
        semantic_version: "1.0.0".into(),
        definition_digest: format!("sha256:{}", "ab".repeat(32)),
        dependency_lock_digest: format!("sha256:{}", "ef".repeat(32)),
        repository: "github.com/acme/standup-bot".into(),
        commit_sha: "a".repeat(40),
        binding_resolution: WorkflowBindingResolution {
            approved_digest: format!("sha256:{}", "ab".repeat(32)),
            executable_digest: format!("sha256:{}", "ab".repeat(32)),
        },
    };
    let value = serde_json::to_value(&response).unwrap();
    assert_eq!(value["workflow"], "daily-standup-report");
    assert_eq!(value["versionId"], format!("sha256:{}", "cd".repeat(32)));
    assert_eq!(value["semanticVersion"], "1.0.0");
    assert_eq!(
        value["definitionDigest"],
        format!("sha256:{}", "ab".repeat(32))
    );
    assert_eq!(
        value["dependencyLockDigest"],
        format!("sha256:{}", "ef".repeat(32))
    );
    assert_eq!(value["repository"], "github.com/acme/standup-bot");
    assert_eq!(
        serde_json::from_value::<WorkflowPublishResponse>(value).unwrap(),
        response
    );
}

#[test]
fn instance_record_round_trip_with_position_and_trigger() {
    let record = WorkflowInstanceRecord {
        instance_id: "8b6d2f4e-70f3-4c92-9d0f-2ad51d4e9c11".into(),
        workflow: "daily-standup-report".into(),
        version_id: format!("sha256:{}", "cd".repeat(32)),
        status: WorkflowInstanceStatus::Paused,
        trigger: Some(WorkflowTriggerSource {
            class: WorkflowTriggerClass::User,
            event_id: None,
        }),
        position: Some(WorkflowRunPositionSummary {
            current_node: Some("step-002".into()),
            steps_taken: 1,
            path: vec!["step-001".into()],
        }),
    };
    assert_eq!(
        serde_json::to_value(&record).unwrap(),
        json!({
            "instanceId": "8b6d2f4e-70f3-4c92-9d0f-2ad51d4e9c11",
            "workflow": "daily-standup-report",
            "versionId": "sha256:".to_owned() + &"cd".repeat(32),
            "status": "paused",
            "trigger": {"class": "user"},
            "position": {"currentNode": "step-002", "stepsTaken": 1, "path": ["step-001"]}
        })
    );
    assert_eq!(
        serde_json::from_value::<WorkflowInstanceRecord>(serde_json::to_value(&record).unwrap())
            .unwrap(),
        record
    );
    let terminal = WorkflowInstanceRunResponse {
        instance_id: "8b6d2f4e-70f3-4c92-9d0f-2ad51d4e9c11".into(),
        workflow: "daily-standup-report".into(),
        version_id: format!("sha256:{}", "cd".repeat(32)),
        status: WorkflowInstanceStatus::Succeeded,
        terminal: WorkflowRunTerminal {
            kind: WorkflowRunTerminalKind::Completed,
            node: None,
            reason: None,
        },
        path: vec!["step-001".into(), "step-002".into()],
    };
    let value = serde_json::to_value(&terminal).unwrap();
    assert_eq!(value["terminal"]["kind"], "completed");
    assert_eq!(
        serde_json::from_value::<WorkflowInstanceRunResponse>(value).unwrap(),
        terminal
    );
}

#[test]
fn review_response_round_trip() {
    let response = WorkflowReviewResponse {
        candidate_id: "cand-7".into(),
        status: WorkflowCandidateStatus::Validated,
        origin: WorkflowTeachMode::Instruct,
        epoch: 0,
        description: Some("taught workflow (1 step(s), mode instruct)".into()),
        steps: vec![WorkflowStepReview {
            node_id: "step-001".into(),
            origin: WorkflowStepOrigin::Instructed,
            description: Some("page the on-call engineer".into()),
            evidence_count: 0,
        }],
        capability_inferences: vec![WorkflowCapabilityInference {
            node_id: "step-001".into(),
            hint: "verb:page".into(),
            rationale: "lexical hint from the instructed step's recorded intent".into(),
        }],
        binding_proposals: vec![WorkflowBindingProposal {
            node_id: "step-001".into(),
            kind: "capability".into(),
            reference: "unresolved:verb:page".into(),
            rationale: "derived from the taught step intent".into(),
            requires_approval: true,
        }],
        trigger_intents: vec![WorkflowTriggerIntent {
            class_hint: "unresolved".into(),
            description: "When the build fails, page the on-call engineer.".into(),
        }],
        validation: WorkflowValidationSummary {
            clean: true,
            error_count: 0,
            warning_count: 0,
            findings: Vec::new(),
        },
        simulation: WorkflowSimulationSummary {
            outcome: WorkflowSimulationOutcomeKind::Completed,
            node: None,
            reason: None,
            path: vec!["step-001".into()],
            steps_taken: 1,
            epoch: 0,
        },
    };
    let value = serde_json::to_value(&response).unwrap();
    assert_eq!(value["status"], "validated");
    assert_eq!(value["steps"][0]["origin"], "instructed");
    assert_eq!(value["bindingProposals"][0]["requiresApproval"], true);
    assert_eq!(value["triggerIntents"][0]["classHint"], "unresolved");
    assert_eq!(
        serde_json::from_value::<WorkflowReviewResponse>(value).unwrap(),
        response
    );
}

#[test]
fn approve_params_reject_unknown_fields_and_decode_decisions() {
    let approved = WorkflowApproveParams {
        candidate_id: "cand-7".into(),
        approver: "tech-lead".into(),
        reference: "review-1".into(),
        decision: WorkflowApprovalDecision::Approved,
    };
    assert_eq!(
        serde_json::to_value(&approved).unwrap(),
        json!({
            "candidateId": "cand-7",
            "approver": "tech-lead",
            "reference": "review-1",
            "decision": "approved"
        })
    );
    assert_eq!(
        serde_json::from_value::<WorkflowApproveParams>(json!({
            "candidateId": "cand-7",
            "approver": "tech-lead",
            "reference": "review-1",
            "decision": "rejected"
        }))
        .unwrap()
        .decision,
        WorkflowApprovalDecision::Rejected
    );
    assert!(
        serde_json::from_value::<WorkflowApproveParams>(json!({
            "candidateId": "cand-7",
            "approver": "tech-lead",
            "reference": "review-1",
            "decision": "maybe"
        }))
        .is_err()
    );
}

#[test]
fn fork_params_round_trip_and_reject_unknown_fields() {
    let params = WorkflowForkParams {
        version_id: format!("sha256:{}", "cd".repeat(32)),
        fork_repository: "local/workflows/forks/daily-standup-report".into(),
        semantic_version: Some("1.1.0".into()),
        commit_sha: Some("a".repeat(40)),
        owner: Some("alice".into()),
        license: Some("Apache-2.0".into()),
        attribution: vec![WorkflowForkAttribution {
            name: "acme-ops".into(),
            contact: Some("acme-ops@example.com".into()),
        }],
    };
    assert_eq!(
        serde_json::to_value(&params).unwrap(),
        json!({
            "versionId": format!("sha256:{}", "cd".repeat(32)),
            "forkRepository": "local/workflows/forks/daily-standup-report",
            "semanticVersion": "1.1.0",
            "commitSha": "a".repeat(40),
            "owner": "alice",
            "license": "Apache-2.0",
            "attribution": [{
                "name": "acme-ops",
                "contact": "acme-ops@example.com"
            }]
        })
    );
    assert_eq!(
        serde_json::from_value::<WorkflowForkParams>(serde_json::to_value(&params).unwrap())
            .unwrap(),
        params
    );
    // Optional identity inputs default to the upstream's; an omitted
    // attribution decodes as empty (the engine refuses it).
    let defaulted = serde_json::from_value::<WorkflowForkParams>(json!({
        "versionId": format!("sha256:{}", "cd".repeat(32)),
        "forkRepository": "local/workflows/forks/daily-standup-report",
        "attribution": []
    }))
    .unwrap();
    assert_eq!(defaulted.semantic_version, None);
    assert_eq!(defaulted.commit_sha, None);
    assert_eq!(defaulted.owner, None);
    assert_eq!(defaulted.license, None);
    assert!(defaulted.attribution.is_empty());
    assert!(
        serde_json::from_value::<WorkflowForkParams>(json!({
            "versionId": format!("sha256:{}", "cd".repeat(32)),
            "forkRepository": "local/workflows/forks/daily-standup-report",
            "attribution": [],
            "unexpected": true
        }))
        .is_err()
    );
}

#[test]
fn fork_response_serializes_lineage_and_attribution() {
    let response = WorkflowForkResponse {
        workflow: "daily-standup-report".into(),
        version_id: format!("sha256:{}", "ef".repeat(32)),
        semantic_version: "1.0.0".into(),
        definition_digest: format!("sha256:{}", "ab".repeat(32)),
        dependency_lock_digest: format!("sha256:{}", "cd".repeat(32)),
        repository: "local/workflows/forks/daily-standup-report".into(),
        commit_sha: "a".repeat(40),
        lineage: WorkflowForkLineage {
            workflow: "daily-standup-report".into(),
            version_id: format!("sha256:{}", "cd".repeat(32)),
            semantic_version: "1.0.0".into(),
            repository: "local/workflows/taught".into(),
            definition_digest: format!("sha256:{}", "ab".repeat(32)),
            dependency_lock_digest: format!("sha256:{}", "cd".repeat(32)),
            commit_sha: "a".repeat(40),
        },
        attribution: vec![WorkflowForkAttribution {
            name: "acme-ops".into(),
            contact: Some("acme-ops@example.com".into()),
        }],
    };
    let value = serde_json::to_value(&response).unwrap();
    assert_eq!(value["workflow"], "daily-standup-report");
    assert_eq!(value["versionId"], format!("sha256:{}", "ef".repeat(32)));
    // The lineage pins the forked-from version id and the upstream digests.
    assert_eq!(
        value["lineage"]["versionId"],
        format!("sha256:{}", "cd".repeat(32))
    );
    assert_eq!(
        value["lineage"]["definitionDigest"],
        format!("sha256:{}", "ab".repeat(32))
    );
    assert_eq!(
        value["lineage"]["dependencyLockDigest"],
        format!("sha256:{}", "cd".repeat(32))
    );
    assert_eq!(value["lineage"]["repository"], "local/workflows/taught");
    // The carried attribution renders.
    assert_eq!(value["attribution"][0]["name"], "acme-ops");
    assert_eq!(value["attribution"][0]["contact"], "acme-ops@example.com");
    assert_eq!(
        serde_json::from_value::<WorkflowForkResponse>(value).unwrap(),
        response
    );
}

#[test]
fn instance_list_params_is_an_empty_object() {
    let params = WorkflowInstanceListParams {};
    assert_eq!(serde_json::to_value(&params).unwrap(), json!({}));
    assert_eq!(
        serde_json::from_value::<WorkflowInstanceListParams>(json!({})).unwrap(),
        params
    );
}
