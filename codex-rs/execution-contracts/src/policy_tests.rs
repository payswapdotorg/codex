use super::*;
use crate::BindingClass;
use crate::ExecutionContractError;
use crate::ExecutionEnvironment;
use pretty_assertions::assert_eq;

#[test]
fn the_default_policy_forbids_human_fallback() {
    let policy = BindingPolicy::default();
    assert_eq!(policy.human_fallback, HumanFallbackPolicy::Forbidden);
    assert!(matches!(policy.environments, EnvironmentScope::All));
    policy.validate().expect("default policy is valid");
    assert!(!policy.allows_class(BindingClass::HumanFallback));
}

#[test]
fn environment_scope_gates_environments() {
    let policy = BindingPolicy {
        environments: EnvironmentScope::Only(vec![
            ExecutionEnvironment::Terminal,
            ExecutionEnvironment::Tool,
        ]),
        human_fallback: HumanFallbackPolicy::Forbidden,
    };
    policy.validate().expect("valid policy");
    assert!(policy.allows_environment(ExecutionEnvironment::Terminal));
    assert!(policy.allows_environment(ExecutionEnvironment::Tool));
    assert!(!policy.allows_environment(ExecutionEnvironment::Browser));
    assert!(!policy.allows_environment(ExecutionEnvironment::Human));
}

#[test]
fn scopes_reject_empty_duplicate_and_reserved_environments() {
    let empty = BindingPolicy {
        environments: EnvironmentScope::Only(vec![]),
        human_fallback: HumanFallbackPolicy::Allowed,
    };
    assert!(empty.validate().is_err());

    let duplicate = BindingPolicy {
        environments: EnvironmentScope::Only(vec![
            ExecutionEnvironment::Api,
            ExecutionEnvironment::Api,
        ]),
        human_fallback: HumanFallbackPolicy::Allowed,
    };
    assert!(duplicate.validate().is_err());

    let reserved = BindingPolicy {
        environments: EnvironmentScope::Only(vec![ExecutionEnvironment::Mobile]),
        human_fallback: HumanFallbackPolicy::Allowed,
    };
    let error = reserved
        .validate()
        .expect_err("reserved environments cannot be scoped in");
    assert!(matches!(
        error,
        ExecutionContractError::InvalidPolicy { .. }
    ));
}

#[test]
fn human_fallback_must_be_explicitly_permitted() {
    let allowed = BindingPolicy {
        environments: EnvironmentScope::All,
        human_fallback: HumanFallbackPolicy::Allowed,
    };
    assert!(allowed.allows_class(BindingClass::HumanFallback));
    assert!(allowed.allows_class(BindingClass::CodexNative));
    assert!(allowed.allows_class(BindingClass::Compatible));

    let forbidden = BindingPolicy::default();
    assert!(!forbidden.allows_class(BindingClass::HumanFallback));
    assert!(forbidden.allows_class(BindingClass::CodexNative));
}

#[test]
fn policies_round_trip_through_serde() {
    let policy = BindingPolicy {
        environments: EnvironmentScope::Only(vec![ExecutionEnvironment::Browser]),
        human_fallback: HumanFallbackPolicy::Allowed,
    };
    let serialized = serde_json::to_string(&policy).expect("serializable");
    let round_tripped: BindingPolicy = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(round_tripped, policy);
}
