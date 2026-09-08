use super::*;
use pretty_assertions::assert_eq;

#[test]
fn provenance_round_trips_through_serde() {
    let provenance = WorkflowProvenance {
        authors: vec![Attribution {
            name: "Ada Lovelace".to_owned(),
            contact: Some("mailto:ada@example.com".to_owned()),
        }],
        forked_from: Some(ForkLineage {
            upstream: WorkflowRepositoryId::parse("https://github.com/acme/upstream-bot")
                .expect("valid repo"),
            forked_at: Some(
                RevisionSha::parse("0123456789abcdef0123456789abcdef01234567").expect("valid sha"),
            ),
        }),
        license: Some(LicenseInfo {
            spdx: "Apache-2.0".to_owned(),
            url: Some("https://www.apache.org/licenses/LICENSE-2.0".to_owned()),
        }),
        upgrade_policy: Some(UpgradePolicy::Prompt),
    };

    let serialized = serde_json::to_string(&provenance).expect("provenance serializes");
    let parsed: WorkflowProvenance = serde_json::from_str(&serialized).expect("provenance parses");

    assert_eq!(provenance, parsed);
}

#[test]
fn provenance_serializes_compactly_when_empty() {
    let serialized =
        serde_json::to_string(&WorkflowProvenance::default()).expect("empty provenance serializes");
    assert_eq!(serialized, "{}");
    assert!(WorkflowProvenance::default().is_empty());
}

#[test]
fn provenance_has_no_commercial_settlement_fields() {
    let schema = serde_json::to_value(&WorkflowProvenance {
        authors: vec![Attribution {
            name: "Ada".to_owned(),
            contact: None,
        }],
        forked_from: None,
        license: None,
        upgrade_policy: Some(UpgradePolicy::Manual),
    })
    .expect("provenance serializes");

    let serialized = schema.to_string();
    for forbidden in [
        "price",
        "payment",
        "billing",
        "credential",
        "secret",
        "token",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "provenance must not carry `{forbidden}` fields, got: {serialized}"
        );
    }
}

#[test]
fn upgrade_policies_serialize_as_camel_case() {
    let policies = [
        (UpgradePolicy::Manual, "\"manual\""),
        (UpgradePolicy::Prompt, "\"prompt\""),
        (
            UpgradePolicy::AutomaticWithinRange,
            "\"automaticWithinRange\"",
        ),
    ];
    for (policy, expected) in policies {
        let serialized = serde_json::to_string(&policy).expect("policy serializes");
        assert_eq!(serialized, expected);
    }
}
