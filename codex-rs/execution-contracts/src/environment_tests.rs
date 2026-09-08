use super::*;
use pretty_assertions::assert_eq;

#[test]
fn wire_names_round_trip_for_every_environment() {
    for environment in ExecutionEnvironment::ALL {
        let parsed =
            ExecutionEnvironment::parse(environment.as_str()).expect("canonical name must parse");
        assert_eq!(parsed, environment);
        let serialized = serde_json::to_string(&environment).expect("serializable");
        let round_tripped: ExecutionEnvironment =
            serde_json::from_str(&serialized).expect("deserializable");
        assert_eq!(round_tripped, environment);
    }
}

#[test]
fn peers_and_reserved_partition_the_known_environments() {
    let mut all = ExecutionEnvironment::PEERS.to_vec();
    all.extend(ExecutionEnvironment::RESERVED);
    all.sort();
    let mut expected = ExecutionEnvironment::ALL.to_vec();
    expected.sort();
    assert_eq!(all, expected);
    assert_eq!(ExecutionEnvironment::PEERS.len(), 7);
    assert_eq!(ExecutionEnvironment::RESERVED.len(), 2);
}

#[test]
fn reserved_environments_are_flagged() {
    assert!(ExecutionEnvironment::Mobile.is_reserved());
    assert!(ExecutionEnvironment::RemoteDesktop.is_reserved());
    for environment in ExecutionEnvironment::PEERS {
        assert!(
            !environment.is_reserved(),
            "{environment} must be a bindable peer"
        );
    }
}

#[test]
fn wire_names_use_camel_case() {
    assert_eq!(ExecutionEnvironment::Browser.as_str(), "browser");
    assert_eq!(ExecutionEnvironment::Mcp.as_str(), "mcp");
    assert_eq!(
        ExecutionEnvironment::RemoteDesktop.as_str(),
        "remoteDesktop"
    );
    assert_eq!(
        ExecutionEnvironment::RemoteDesktop.to_string(),
        "remoteDesktop"
    );
}

#[test]
fn parsing_rejects_unknown_or_cased_names() {
    for value in ["BROWSER", "browser ", "device", ""] {
        assert!(
            ExecutionEnvironment::parse(value).is_err(),
            "`{value}` must be rejected"
        );
    }
}
