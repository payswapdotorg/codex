use pretty_assertions::assert_eq;
use std::time::Duration;

use super::*;
use crate::capabilities::ModelCapabilities;
use crate::capabilities::ModelFeature;
use crate::capabilities::UnsupportedModelCapability;
use codex_protocol::openai_models::ReasoningEffort;

#[test]
fn classifies_retryability() {
    let retryable = [
        ModelError::Transport {
            message: "timeout".to_string(),
            retryable: true,
            retry_after: None,
            status: None,
        },
        ModelError::RateLimited {
            message: "429".to_string(),
            retry_after: Some(Duration::from_secs(2)),
        },
        ModelError::Overloaded {
            message: "server overloaded".to_string(),
        },
    ];
    let terminal = [
        ModelError::Auth {
            message: "unauthorized".to_string(),
        },
        ModelError::InvalidRequest {
            message: "bad request".to_string(),
        },
        ModelError::ContextWindowExceeded {
            message: "too long".to_string(),
        },
        ModelError::UsageLimitExceeded {
            message: "quota".to_string(),
        },
        ModelError::StreamIncomplete {
            message: "closed".to_string(),
        },
        ModelError::Canceled {
            reason: "interrupt".to_string(),
        },
        ModelError::Provider {
            status: Some(400),
            code: None,
            message: "unknown".to_string(),
        },
    ];

    for error in &retryable {
        assert!(error.is_retryable(), "{error} should be retryable");
    }
    for error in &terminal {
        assert!(!error.is_retryable(), "{error} should not be retryable");
    }
}

#[test]
fn unsupported_capability_carries_diagnostics() {
    let capabilities = ModelCapabilities {
        provider_id: "openai".to_string(),
        model_id: "gpt-5.6-luna".to_string(),
        supported_reasoning_efforts: vec![ReasoningEffort::Medium],
        ..ModelCapabilities::default()
    };
    let failure: ModelError = capabilities
        .require(ModelFeature::ReasoningEffort(ReasoningEffort::Max))
        .expect_err("unsupported effort should fail")
        .into();

    assert_eq!(
        failure,
        ModelError::UnsupportedCapability(UnsupportedModelCapability {
            feature: ModelFeature::ReasoningEffort(ReasoningEffort::Max),
            provider_id: "openai".to_string(),
            model_id: "gpt-5.6-luna".to_string(),
            message: "model openai/gpt-5.6-luna does not support reasoning effort max".to_string(),
            supported_alternatives: vec!["medium".to_string()],
        })
    );
    assert!(!failure.is_retryable());
    assert!(
        failure.to_string().contains("supported: medium"),
        "error should render diagnostics: {failure}"
    );
}

#[test]
fn exposes_http_status_when_present() {
    let transport = ModelError::Transport {
        message: "http 503".to_string(),
        retryable: true,
        retry_after: None,
        status: Some(503),
    };
    let provider = ModelError::Provider {
        status: Some(404),
        code: Some("model_not_found".to_string()),
        message: "model gone".to_string(),
    };

    assert_eq!(transport.status(), Some(503));
    assert_eq!(provider.status(), Some(404));
    assert_eq!(
        ModelError::Canceled {
            reason: "user".to_string()
        }
        .status(),
        None
    );
}
