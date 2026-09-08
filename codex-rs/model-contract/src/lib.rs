//! Frozen provider-neutral model contract for the Codex universal model plane.
//!
//! This crate is the universal boundary between the Codex runtime and model
//! providers. It is the only layer that defines how the runtime describes,
//! addresses, invokes, and observes models without depending on any provider
//! protocol, SDK, transport, or configuration format.
//!
//! # Frozen boundary rules
//!
//! The contract is frozen by `docs/architecture/CODEX-UNIVERSAL-ARCHITECTURE.md`
//! (universal model plane, architecture v0.2.0):
//!
//! - **No provider protocol types.** Types from provider SDKs, wire protocols,
//!   HTTP clients, or provider-specific request/response shapes must not
//!   appear in this crate or its public API. Provider adapters convert between
//!   their protocol and the types defined here.
//! - **No credentials.** Contract types never carry API keys, tokens, or other
//!   secrets. Authentication is represented only through normalized status
//!   ([`ModelAuthStatus`]).
//! - **Opaque model identity.** [`ModelDescriptor`] records provider and model
//!   identity as opaque strings only. Durable thread/workflow state must not
//!   record endpoints, wire configuration, or auth details.
//! - **Provider swappability.** Switching the provider that serves a thread
//!   changes only the descriptor of a request, never its semantic content
//!   (instructions, input items, tools).
//! - **Model capabilities are distinct from runtime capabilities.** Browser,
//!   desktop, shell, MCP, skills, and similar execution capabilities stay
//!   available to any model; [`ModelCapabilities`] describes what the model
//!   itself supports.
//! - **Explicit capability failures.** Requesting a feature a model does not
//!   support must fail with [`UnsupportedModelCapability`] diagnostics instead
//!   of silently degrading.
//!
//! # Contract surface
//!
//! The eight frozen contract types are:
//!
//! - [`ModelProvider`] — provider boundary: identity, transport policy,
//!   authentication status, model capabilities, and session creation.
//! - [`ModelDescriptor`] — provider-neutral model identity.
//! - [`ModelCapabilities`] — effective model feature support with
//!   negotiation.
//! - [`ModelRequest`] — one model invocation request.
//! - [`ModelResponse`] — summary of a completed invocation.
//! - [`ModelEvent`] — normalized streaming events for an invocation.
//! - [`ModelError`] — normalized provider failure taxonomy.
//! - [`ModelSession`] — per-turn execution boundary with normalized
//!   streaming and cancellation.
//!
//! Conversation content ([`codex_protocol::models::ResponseItem`]) and tool
//! declarations ([`codex_tools::ToolSpec`]) are the shared Codex runtime
//! currency and are referenced, not redefined, by this contract.

mod capabilities;
mod descriptor;
mod error;
mod event;
mod provider;
mod request;
mod response;
mod session;

pub use capabilities::ModelCapabilities;
pub use capabilities::ModelFeature;
pub use capabilities::RemoteCompactionSupport;
pub use capabilities::UnsupportedModelCapability;
pub use descriptor::ModelDescriptor;
pub use error::ModelError;
pub use event::ModelEvent;
pub use event::ModelSafetyBuffering;
pub use provider::ModelAuthStatusFuture;
pub use provider::ModelProvider;
pub use request::ModelReasoningControls;
pub use request::ModelRequest;
pub use request::ModelToolChoice;
pub use response::ModelResponse;
pub use session::ModelAuthStatus;
pub use session::ModelSession;
pub use session::ModelSessionFuture;
pub use session::ModelStream;
pub use session::ModelStreamItem;
pub use session::ModelTransportPolicy;
