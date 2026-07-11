//! Real-time events (P6): the SSE event envelope and the in-process fan-out
//! hub used to deliver targeted and broadcast events to subscribers.

pub mod envelope;
pub mod hub;

pub use envelope::EventEnvelope;
pub use hub::{EventHub, SequencedEvent, Subscription};
