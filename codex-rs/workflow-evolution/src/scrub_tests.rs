//! Scrubbing and credential-check tests.

use pretty_assertions::assert_eq;
use serde_json::json;

use super::REDACTED_MARKER;
use super::require_clean_json;
use super::scrub_json;

#[test]
fn clean_values_pass_untouched() {
    let value = json!({
        "description": "governed evolution from 2 recorded traces",
        "purpose": "inspect the issues list (evidence: 2 observations)",
        "counts": [1, 2, 3],
    });
    let report = scrub_json(&value);
    assert_eq!(
        report,
        super::ScrubReport {
            clean: value.clone(),
            findings: Vec::new(),
        }
    );
    assert!(require_clean_json(&value, "test").is_ok());
}

#[test]
fn secret_keys_are_redacted_and_flagged() {
    let value = json!({
        "settings": {
            "api_key": "any-nonempty-value",
            "refresh-token": "also-secret",
            "target": "issues.example.com",
        }
    });
    let report = scrub_json(&value);
    assert_eq!(report.findings.len(), 2);
    assert!(
        report
            .findings
            .iter()
            .all(|finding| finding.reason == "secret-bearing key")
    );
    assert_eq!(
        report.clean,
        json!({
            "settings": {
                "api_key": REDACTED_MARKER,
                "refresh-token": REDACTED_MARKER,
                "target": "issues.example.com",
            }
        })
    );
    assert!(matches!(
        require_clean_json(&value, "test"),
        Err(crate::WorkflowEvolutionError::CredentialContamination { findings: 2, .. })
    ));
}

#[test]
fn credential_shapes_are_detected_in_values() {
    // Synthetic OpenAI-style key shape, assembled at runtime from fragments
    // so the literal never appears in the source text (secret scanners and
    // GitHub push protection pattern-match full key shapes even in test
    // fixtures; the scrubber itself sees the complete shape at runtime).
    let synthetic_openai_key = format!("sk-{}-{}", "proj", "abcdefghijklmnopqrstuvwx");
    // Same runtime-assembly treatment for the Slack marker literal (the
    // scanner flags the full placeholder string too).
    // Synthetic Slack token shape, assembled at runtime from fragments
    // so the literal never appears in the source text.
    let synthetic_slack_token = format!(
        "{}{}",
        "xox", "b-1234567890-1234567890-1234567890-abcdefghijklmnopqrstuvwx"
    );
    let shapes = [
        (
            "-----BEGIN RSA PRIVATE KEY-----\nMIIE...\n-----END RSA PRIVATE KEY-----",
            "PEM private-key block",
        ),
        ("AKIAIOSFODNN7EXAMPLE", "AWS access key id"),
        (synthetic_openai_key.as_str(), "OpenAI-style secret key"),
        ("ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890", "GitHub token"),
        (synthetic_slack_token.as_str(), "Slack token"),
        (
            "https://alice:ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ12@github.com/org/repo",
            "URL-embedded credential",
        ),
    ];
    for (value, reason) in shapes {
        let report = scrub_json(&json!({ "source": value }));
        assert_eq!(report.findings.len(), 1, "shape must be flagged: {value}");
        assert_eq!(report.findings[0].reason, reason);
        assert_eq!(
            report.clean,
            json!({ "source": REDACTED_MARKER }),
            "flagged value must be redacted: {value}"
        );
    }
}

#[test]
fn benign_values_are_not_flagged() {
    // Short strings never carry the scanned shapes; ordinary URLs without
    // embedded credentials, human-facing text, and hyphenated words pass
    // untouched.
    let value = json!({
        "token": "",
        "repository": "https://github.com/acme/ops-workflows",
        "note": "looks good to me",
        "short": "sk-123",
        "task": "task-runner",
        "schedule": "risk-management window",
    });
    let report = scrub_json(&value);
    assert!(report.is_clean());
}

#[test]
fn credential_tokens_are_detected_mid_string() {
    // Free text carrying a pasted token anywhere is flagged, while
    // hyphenated words are not.
    let flagged = "rotate the key sk-proj-abcdefghijklmnopqrstuvwx soon";
    let report = scrub_json(&json!({ "rationale": flagged }));
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].reason, "OpenAI-style secret key");
    let benign = json!({ "rationale": "finish the task-runner step next" });
    assert!(scrub_json(&benign).is_clean());
}

#[test]
fn findings_record_paths_but_never_content() {
    let value = json!({ "outer": [{ "password": "hunter2hunter2" }] });
    let report = scrub_json(&value);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].path, "$.outer[0].password");
    let rendered = format!("{:?}", report.findings);
    assert!(
        !rendered.contains("hunter2"),
        "findings must never echo the matched content"
    );
}
