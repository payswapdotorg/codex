use pretty_assertions::assert_eq;
use serde_json::json;

use super::*;

fn descriptor() -> ModelDescriptor {
    ModelDescriptor::new("openai", "gpt-5.6-luna").with_display_name("GPT-5.6 Luna")
}

#[test]
fn descriptor_serializes_only_opaque_identity() {
    let value = serde_json::to_value(descriptor()).expect("descriptor should serialize");

    assert_eq!(
        value,
        json!({
            "provider_id": "openai",
            "model_id": "gpt-5.6-luna",
            "display_name": "GPT-5.6 Luna",
        })
    );
}

#[test]
fn descriptor_round_trips_through_serde() {
    let original = descriptor();
    let encoded = serde_json::to_string(&original).expect("descriptor should serialize");
    let decoded: ModelDescriptor =
        serde_json::from_str(&encoded).expect("descriptor should deserialize");

    assert_eq!(decoded, original);
}

#[test]
fn swapping_providers_changes_only_the_provider_field() {
    // The same durable thread can be re-pointed at a different provider;
    // only the provider identity changes in the recorded descriptor.
    let openai = ModelDescriptor::new("openai", "gpt-5.6-luna");
    let other = ModelDescriptor::new("amazon-bedrock", "gpt-5.6-luna");

    let openai_value = serde_json::to_value(&openai).expect("openai descriptor should serialize");
    let other_value = serde_json::to_value(&other).expect("other descriptor should serialize");

    // Durable thread state references models as opaque identifiers; switching
    // providers must change only the provider identifier, never add provider
    // protocol, endpoint, or credential fields.
    assert_eq!(openai_value.get("model_id"), other_value.get("model_id"));
    assert_ne!(
        openai_value.get("provider_id"),
        other_value.get("provider_id")
    );
    assert_eq!(
        openai_value
            .as_object()
            .expect("descriptor is an object")
            .len(),
        other_value
            .as_object()
            .expect("descriptor is an object")
            .len()
    );
}

#[test]
fn descriptor_display_is_qualified_id() {
    assert_eq!(descriptor().to_string(), "openai/gpt-5.6-luna");
    assert_eq!(descriptor().qualified_id(), "openai/gpt-5.6-luna");
}
