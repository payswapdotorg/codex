//! JSON-RPC integration tests for the workflow control-plane mount
//! (RWO-001): the teach -> compile -> review -> approve -> publish
//! pipeline and the durable instance lifecycle, driven through the real
//! app-server exactly the way an app-server client would.

use anyhow::Result;
use app_test_support::DEFAULT_CLIENT_NAME;
use app_test_support::TestAppServer;
use codex_app_server_protocol::ClientInfo;
use codex_app_server_protocol::InitializeCapabilities;
use codex_app_server_protocol::RequestId;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;
use tokio::time::Duration;
use tokio::time::timeout;

const COMMIT_SHA: &str = "0123456789abcdef0123456789abcdef01234567";

async fn initialize_experimental(app_server: &mut TestAppServer) -> Result<()> {
    app_server
        .initialize_with_capabilities(
            ClientInfo {
                name: DEFAULT_CLIENT_NAME.into(),
                title: None,
                version: "0.1.0".into(),
            },
            Some(InitializeCapabilities {
                experimental_api: true,
                ..Default::default()
            }),
        )
        .await?;
    Ok(())
}

async fn request(app_server: &mut TestAppServer, method: &str, params: Value) -> Result<Value> {
    let id = app_server.send_raw_request(method, Some(params)).await?;
    let response = timeout(
        Duration::from_secs(30),
        app_server.read_stream_until_response_message(RequestId::Integer(id)),
    )
    .await??;
    Ok(response.result)
}

async fn request_error(
    app_server: &mut TestAppServer,
    method: &str,
    params: Value,
) -> Result<codex_app_server_protocol::JSONRPCErrorError> {
    let id = app_server.send_raw_request(method, Some(params)).await?;
    let response = timeout(
        Duration::from_secs(30),
        app_server.read_stream_until_error_message(RequestId::Integer(id)),
    )
    .await??;
    Ok(response.error)
}

/// Drives one full teaching pipeline and returns the publish result.
async fn teach_publish(
    app_server: &mut TestAppServer,
    mode: &str,
    name: &str,
    instruction: Option<&str>,
    demonstration: Option<&str>,
) -> Result<Value> {
    let start = request(
        app_server,
        "workflow/teach/start",
        json!({"mode": mode, "name": name}),
    )
    .await?;
    let session_id = start["sessionId"].as_str().expect("session id").to_string();
    if let Some(instruction) = instruction {
        request(
            app_server,
            "workflow/teach/instruct",
            json!({"sessionId": session_id, "text": instruction}),
        )
        .await?;
    }
    if let Some(demonstration) = demonstration {
        request(
            app_server,
            "workflow/teach/demonstrate",
            json!({"sessionId": session_id, "kind": "action", "text": demonstration}),
        )
        .await?;
    }
    request(
        app_server,
        "workflow/teach/reconcile",
        json!({"sessionId": session_id}),
    )
    .await?;
    let compiled = request(
        app_server,
        "workflow/compile",
        json!({"sessionId": session_id}),
    )
    .await?;
    assert_eq!(compiled["status"], "validated");
    let candidate_id = compiled["candidateId"]
        .as_str()
        .expect("candidate id")
        .to_string();
    let reviewed = request(
        app_server,
        "workflow/review",
        json!({"candidateId": candidate_id}),
    )
    .await?;
    assert_eq!(reviewed["status"], "validated");
    assert!(reviewed["steps"].as_array().expect("steps").len() >= 1);
    assert!(
        reviewed["bindingProposals"]
            .as_array()
            .expect("binding proposals")
            .len()
            >= 1
    );
    let approved = request(
        app_server,
        "workflow/approve",
        json!({
            "candidateId": candidate_id,
            "approver": "tech-lead",
            "reference": "review-1",
            "decision": "approved"
        }),
    )
    .await?;
    assert_eq!(approved["status"], "approved");
    request(
        app_server,
        "workflow/publish",
        json!({
            "candidateId": candidate_id,
            "commitSha": COMMIT_SHA
        }),
    )
    .await
}

#[tokio::test]
async fn workflow_methods_require_experimental_api_capability() -> Result<()> {
    let mut app_server = TestAppServer::builder().build().await?;
    app_server
        .initialize_with_capabilities(
            ClientInfo {
                name: DEFAULT_CLIENT_NAME.into(),
                title: None,
                version: "0.1.0".into(),
            },
            Some(InitializeCapabilities {
                experimental_api: false,
                ..Default::default()
            }),
        )
        .await?;
    let error = request_error(
        &mut app_server,
        "workflow/teach/start",
        json!({"mode": "instruct"}),
    )
    .await?;
    assert_eq!(
        error,
        codex_app_server_protocol::JSONRPCErrorError {
            code: -32600,
            message: "workflow/teach/start requires experimentalApi capability".into(),
            data: None,
        }
    );
    Ok(())
}

#[tokio::test]
async fn teach_instruct_compile_review_approve_publish_over_jsonrpc() -> Result<()> {
    let mut app_server = TestAppServer::builder().build().await?;
    initialize_experimental(&mut app_server).await?;
    let published = teach_publish(
        &mut app_server,
        "instruct",
        "daily-standup-report",
        Some("Summarize the daily progress report."),
        None,
    )
    .await?;
    assert_eq!(published["workflow"], "daily-standup-report");
    assert_eq!(published["semanticVersion"], "1.0.0");
    assert_eq!(published["repository"], "local/workflows/taught");
    assert_eq!(published["commitSha"], COMMIT_SHA);
    assert!(
        published["versionId"]
            .as_str()
            .expect("version id")
            .starts_with("sha256:")
    );
    assert!(
        published["definitionDigest"]
            .as_str()
            .expect("definition digest")
            .starts_with("sha256:")
    );
    assert!(
        published["dependencyLockDigest"]
            .as_str()
            .expect("lock digest")
            .starts_with("sha256:")
    );
    Ok(())
}

#[tokio::test]
async fn hybrid_teaching_reconciles_instruction_and_demonstration() -> Result<()> {
    let mut app_server = TestAppServer::builder().build().await?;
    initialize_experimental(&mut app_server).await?;
    let start = request(
        &mut app_server,
        "workflow/teach/start",
        json!({"mode": "hybrid", "name": "media-content-repurposing"}),
    )
    .await?;
    let session_id = start["sessionId"].as_str().expect("session id").to_string();
    request(
        &mut app_server,
        "workflow/teach/instruct",
        json!({"sessionId": session_id, "text": "Publish the weekly digest on Mondays."}),
    )
    .await?;
    request(
        &mut app_server,
        "workflow/teach/demonstrate",
        json!({"sessionId": session_id, "kind": "action", "text": "Open the content queue."}),
    )
    .await?;
    let reconciled = request(
        &mut app_server,
        "workflow/teach/reconcile",
        json!({"sessionId": session_id}),
    )
    .await?;
    // Instruction and demonstration are reconciled, not one dropped.
    assert_eq!(reconciled["demonstrationRecords"], 1);
    assert_eq!(reconciled["instructionRecords"], 1);
    let compiled = request(
        &mut app_server,
        "workflow/compile",
        json!({"sessionId": session_id}),
    )
    .await?;
    assert_eq!(compiled["stepCount"], 2);
    let candidate_id = compiled["candidateId"]
        .as_str()
        .expect("candidate id")
        .to_string();
    let reviewed = request(
        &mut app_server,
        "workflow/review",
        json!({"candidateId": candidate_id}),
    )
    .await?;
    let origins: Vec<&str> = reviewed["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|step| step["origin"].as_str().expect("origin"))
        .collect();
    assert_eq!(origins, ["observed", "instructed"]);
    Ok(())
}

#[tokio::test]
async fn instances_run_list_get_and_survive_restart_over_jsonrpc() -> Result<()> {
    let home = TempDir::new()?;
    let mut app_server = TestAppServer::builder()
        .with_codex_home(home.path())
        .build()
        .await?;
    initialize_experimental(&mut app_server).await?;
    let published = teach_publish(
        &mut app_server,
        "demonstrate",
        "construction-daily-progress",
        None,
        Some("Open the site log."),
    )
    .await?;
    let version_id = published["versionId"]
        .as_str()
        .expect("version id")
        .to_string();

    let run = request(
        &mut app_server,
        "workflow/instance/run",
        json!({"versionId": version_id}),
    )
    .await?;
    assert_eq!(run["status"], "succeeded");
    assert_eq!(run["terminal"]["kind"], "completed");
    assert_eq!(run["workflow"], "construction-daily-progress");
    let instance_id = run["instanceId"].as_str().expect("instance id").to_string();

    let listed = request(&mut app_server, "workflow/instance/list", json!({})).await?;
    assert_eq!(listed["instances"].as_array().expect("instances").len(), 1);

    let got = request(
        &mut app_server,
        "workflow/instance/get",
        json!({"instanceId": instance_id}),
    )
    .await?;
    assert_eq!(got["instance"]["status"], "succeeded");
    assert_eq!(got["instance"]["workflow"], "construction-daily-progress");

    // The kill/restart story: a brand-new app-server process over the
    // same home still observes the durable instance and its version pin.
    app_server.shutdown_gracefully().await?;
    let mut restarted = TestAppServer::builder()
        .with_codex_home(home.path())
        .build()
        .await?;
    initialize_experimental(&mut restarted).await?;
    let listed = request(&mut restarted, "workflow/instance/list", json!({})).await?;
    let instances = listed["instances"].as_array().expect("instances");
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0]["status"], "succeeded");
    assert_eq!(instances[0]["workflow"], "construction-daily-progress");
    Ok(())
}

#[tokio::test]
async fn unknown_records_surface_as_invalid_params() -> Result<()> {
    let mut app_server = TestAppServer::builder().build().await?;
    initialize_experimental(&mut app_server).await?;
    let error = request_error(
        &mut app_server,
        "workflow/teach/instruct",
        json!({"sessionId": "ws-none", "text": "Nothing."}),
    )
    .await?;
    assert_eq!(error.code, -32602);
    assert!(
        error.message.contains("unknown teaching session"),
        "unexpected message: {}",
        error.message
    );
    let error = request_error(
        &mut app_server,
        "workflow/instance/get",
        json!({"instanceId": "8b6d2f4e-70f3-4c92-9d0f-2ad51d4e9c11"}),
    )
    .await?;
    assert_eq!(error.code, -32602);
    assert!(
        error.message.contains("run position") || error.message.contains("unknown"),
        "unexpected message: {}",
        error.message
    );
    Ok(())
}

#[tokio::test]
async fn fork_publishes_a_new_immutable_release_with_lineage_over_jsonrpc() -> Result<()> {
    let mut app_server = TestAppServer::builder().build().await?;
    initialize_experimental(&mut app_server).await?;
    let published = teach_publish(
        &mut app_server,
        "instruct",
        "forkable-report",
        Some("Summarize the daily progress report."),
        None,
    )
    .await?;
    let version_id = published["versionId"]
        .as_str()
        .expect("version id")
        .to_string();
    let fork_request = json!({
        "versionId": version_id,
        "forkRepository": "local/workflows/forks/forkable-report",
        "attribution": [{"name": "tech-lead", "contact": "tech-lead@example.com"}]
    });

    let forked = request(&mut app_server, "workflow/fork", fork_request.clone()).await?;
    // A NEW immutable release: different version identity, inherited
    // definition and lock digests, the fork's repository.
    assert_ne!(forked["versionId"], published["versionId"]);
    assert_eq!(forked["workflow"], "forkable-report");
    assert_eq!(forked["semanticVersion"], published["semanticVersion"]);
    assert_eq!(forked["definitionDigest"], published["definitionDigest"]);
    assert_eq!(
        forked["dependencyLockDigest"],
        published["dependencyLockDigest"]
    );
    assert_eq!(
        forked["repository"],
        "local/workflows/forks/forkable-report"
    );
    assert_eq!(forked["commitSha"], published["commitSha"]);
    // The upstream is pinned in the lineage record: forked-from id plus
    // the upstream digests.
    assert_eq!(forked["lineage"]["versionId"], published["versionId"]);
    assert_eq!(forked["lineage"]["workflow"], "forkable-report");
    assert_eq!(forked["lineage"]["repository"], published["repository"]);
    assert_eq!(
        forked["lineage"]["definitionDigest"],
        published["definitionDigest"]
    );
    assert_eq!(
        forked["lineage"]["dependencyLockDigest"],
        published["dependencyLockDigest"]
    );
    // The carried attribution renders on the release.
    assert_eq!(forked["attribution"][0]["name"], "tech-lead");
    assert_eq!(forked["attribution"][0]["contact"], "tech-lead@example.com");

    // Re-publishing the same fork identity is refused (invalid params,
    // never a silent overwrite).
    let refused = request_error(&mut app_server, "workflow/fork", fork_request).await?;
    assert_eq!(refused.code, -32602);
    assert!(
        refused.message.contains("already published"),
        "unexpected message: {}",
        refused.message
    );

    // An empty carried attribution is refused by the engine's rule
    // ("a fork must carry upstream attribution").
    let unattributed = request_error(
        &mut app_server,
        "workflow/fork",
        json!({
            "versionId": version_id,
            "forkRepository": "local/workflows/forks/unattributed",
            "attribution": []
        }),
    )
    .await?;
    assert_eq!(unattributed.code, -32602);
    assert!(
        unattributed
            .message
            .contains("must carry upstream attribution"),
        "unexpected message: {}",
        unattributed.message
    );

    // The fork release is real and runnable: an instance pins it.
    let run = request(
        &mut app_server,
        "workflow/instance/run",
        json!({"versionId": forked["versionId"]}),
    )
    .await?;
    assert_eq!(run["status"], "succeeded");
    assert_eq!(run["terminal"]["kind"], "completed");
    assert_eq!(run["workflow"], "forkable-report");
    Ok(())
}

#[tokio::test]
async fn improvement_lifecycle_is_governed_over_jsonrpc() -> Result<()> {
    let mut app_server = TestAppServer::builder().build().await?;
    initialize_experimental(&mut app_server).await?;
    let published = teach_publish(
        &mut app_server,
        "instruct",
        "improvable-standup-report",
        Some("Summarize the daily progress report."),
        None,
    )
    .await?;
    let version_id = published["versionId"]
        .as_str()
        .expect("version id")
        .to_string();

    // Evidence -> candidate: the proposal records execution evidence and
    // returns candidates that cite it.
    let proposal = request(
        &mut app_server,
        "workflow/improve/propose",
        json!({"versionId": version_id}),
    )
    .await?;
    assert_eq!(proposal["workflow"], "improvable-standup-report");
    assert_eq!(proposal["incumbentVersionId"], version_id);
    assert_eq!(proposal["incumbentSemanticVersion"], "1.0.0");
    assert_eq!(
        proposal["evidence"]["references"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(proposal["evidence"]["references"][0]["kind"], "trace");
    assert_eq!(
        proposal["evidence"]["runs"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(proposal["evidence"]["runs"][0]["versionId"], version_id);
    assert_eq!(proposal["evidence"]["runs"][0]["status"], "succeeded");
    let candidates = proposal["candidates"].as_array().expect("candidates");
    assert!(!candidates.is_empty());
    for candidate in candidates {
        // Every candidate cites the recorded evidence.
        assert_eq!(candidate["evidence"], proposal["evidence"]);
        assert_eq!(candidate["incumbentVersionId"], version_id);
    }
    let candidate = candidates
        .iter()
        .find(|candidate| candidate["changeKind"] == "definitionDelta")
        .expect("definition-delta candidate");
    let candidate_id = candidate["candidateId"]
        .as_str()
        .expect("candidate id")
        .to_string();

    // Publishing before validating is refused.
    let unvalidated = request_error(
        &mut app_server,
        "workflow/improve/publish",
        json!({"candidateId": candidate_id, "releaseTag": "improve-1.0.1"}),
    )
    .await?;
    assert_eq!(unvalidated.code, -32602);
    assert!(
        unvalidated.message.contains("no validation report yet"),
        "unexpected message: {}",
        unvalidated.message
    );

    // The validation step: replay, differential, policy.
    let validated = request(
        &mut app_server,
        "workflow/improve/validate",
        json!({"candidateId": candidate_id, "successorVersion": "1.0.1"}),
    )
    .await?;
    assert_eq!(validated["passed"], true);
    let stages = validated["stages"].as_array().expect("stages");
    assert_eq!(stages.len(), 3);
    assert_eq!(stages[0]["stage"], "replay");
    assert_eq!(stages[1]["stage"], "differential");
    assert_eq!(stages[2]["stage"], "policy");
    assert!(stages.iter().all(|stage| stage["passed"] == true));

    // THE APPROVAL GATE: publication without an approval decision is
    // refused over JSON-RPC exactly as the engine refuses it.
    let unapproved = request_error(
        &mut app_server,
        "workflow/improve/publish",
        json!({"candidateId": candidate_id, "releaseTag": "improve-1.0.1"}),
    )
    .await?;
    assert_eq!(unapproved.code, -32602);
    assert!(
        unapproved.message.contains("no approval decision"),
        "unexpected message: {}",
        unapproved.message
    );

    // The explicit approval decision.
    let approved = request(
        &mut app_server,
        "workflow/improve/approve",
        json!({
            "candidateId": candidate_id,
            "approver": "tech-lead",
            "decision": "approved",
            "note": "evidence-cited description refresh looks right"
        }),
    )
    .await?;
    assert_eq!(approved["approved"], true);
    assert_eq!(approved["approver"], "tech-lead");
    assert_eq!(approved["incumbentVersionId"], version_id);
    assert!(
        approved["validationDigest"]
            .as_str()
            .expect("validation digest")
            .starts_with("sha256:")
    );

    // After approval, the improvement lands as a new immutable version
    // with its governed lineage.
    let improvement = request(
        &mut app_server,
        "workflow/improve/publish",
        json!({"candidateId": candidate_id, "releaseTag": "improve-1.0.1"}),
    )
    .await?;
    assert_ne!(improvement["versionId"], published["versionId"]);
    assert_eq!(improvement["semanticVersion"], "1.0.1");
    assert_ne!(
        improvement["definitionDigest"],
        published["definitionDigest"]
    );
    assert_eq!(
        improvement["dependencyLockDigest"],
        published["dependencyLockDigest"]
    );
    assert_eq!(improvement["workflow"], "improvable-standup-report");
    assert_eq!(
        improvement["lineage"]["predecessorVersionId"],
        published["versionId"]
    );
    assert_eq!(
        improvement["lineage"]["successorVersionId"],
        improvement["versionId"]
    );
    assert_eq!(improvement["lineage"]["candidateId"], candidate_id);
    assert_eq!(improvement["lineage"]["approver"], "tech-lead");
    assert_eq!(improvement["lineage"]["releaseTag"], "improve-1.0.1");
    assert_eq!(improvement["evidence"], proposal["evidence"]);

    // The successor is a real immutable release instances can pin.
    let run = request(
        &mut app_server,
        "workflow/instance/run",
        json!({"versionId": improvement["versionId"]}),
    )
    .await?;
    assert_eq!(run["status"], "succeeded");
    assert_eq!(run["terminal"]["kind"], "completed");
    assert_eq!(run["workflow"], "improvable-standup-report");
    Ok(())
}

#[tokio::test]
async fn improvement_rejection_refuses_publication_over_jsonrpc() -> Result<()> {
    let mut app_server = TestAppServer::builder().build().await?;
    initialize_experimental(&mut app_server).await?;
    let published = teach_publish(
        &mut app_server,
        "instruct",
        "rejected-improvement-report",
        Some("Summarize the daily progress report."),
        None,
    )
    .await?;
    let proposal = request(
        &mut app_server,
        "workflow/improve/propose",
        json!({"versionId": published["versionId"]}),
    )
    .await?;
    let candidate_id = proposal["candidates"][0]["candidateId"]
        .as_str()
        .expect("candidate id")
        .to_string();
    request(
        &mut app_server,
        "workflow/improve/validate",
        json!({"candidateId": candidate_id, "successorVersion": "1.0.1"}),
    )
    .await?;
    // A rejection without a reason is an input error.
    let unreasoned = request_error(
        &mut app_server,
        "workflow/improve/approve",
        json!({"candidateId": candidate_id, "approver": "tech-lead", "decision": "rejected"}),
    )
    .await?;
    assert_eq!(unreasoned.code, -32602);
    assert!(
        unreasoned.message.contains("must carry a reason"),
        "unexpected message: {}",
        unreasoned.message
    );
    // The recorded rejection refuses publication.
    let rejected = request(
        &mut app_server,
        "workflow/improve/approve",
        json!({
            "candidateId": candidate_id,
            "approver": "tech-lead",
            "decision": "rejected",
            "reason": "the successor description loses operator context"
        }),
    )
    .await?;
    assert_eq!(rejected["approved"], false);
    assert_eq!(
        rejected["reason"],
        "the successor description loses operator context"
    );
    let refused = request_error(
        &mut app_server,
        "workflow/improve/publish",
        json!({"candidateId": candidate_id, "releaseTag": "improve-rejected"}),
    )
    .await?;
    assert_eq!(refused.code, -32602);
    assert!(
        refused.message.contains("was rejected by approver"),
        "unexpected message: {}",
        refused.message
    );
    Ok(())
}
