//! Asynchronous messaging (TR-06-004…007).
//!
//! Domain messages are produced to topics named `superapp.{service}.{action}`
//! as JSON with a metadata envelope, and consumed via **consumer groups** (one
//! per service) so instances of a service share the load without
//! double-processing. Messages that keep failing are retried and routed to a
//! dead-letter queue ([`dlq`]).
//!
//! Per the project DI rule, producers/consumers depend on the [`MessageBus`]
//! trait. The [`InMemoryBus`] here models topic fan-out + consumer-group
//! competitive delivery faithfully (MPMC channels) and backs all tests; a
//! Kafka wire adapter is a drop-in for deployment (no `rdkafka` on rustc 1.85).

pub mod dlq;

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Topic name for `service`/`action`: `superapp.{service}.{action}` (TR-06-004).
#[must_use]
pub fn topic_name(service: &str, action: &str) -> String {
    format!("superapp.{service}.{action}")
}

/// The dead-letter topic derived from a topic (TR-06-006).
#[must_use]
pub fn dlq_topic(topic: &str) -> String {
    format!("{topic}.dlq")
}

/// Message metadata envelope (TR-06-004).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    /// RFC 3339 produce time.
    pub timestamp: String,
    /// Delivery attempt count (>=1).
    #[serde(default)]
    pub attempt: u32,
    /// Free-form headers.
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

/// A message on the bus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// Unique message id.
    pub id: String,
    pub service: String,
    pub action: String,
    /// JSON payload.
    pub payload: serde_json::Value,
    pub metadata: Metadata,
}

impl Message {
    /// Build a message for `service`/`action` with `payload`.
    #[must_use]
    pub fn new(service: &str, action: &str, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            service: service.to_string(),
            action: action.to_string(),
            payload,
            metadata: Metadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                attempt: 1,
                headers: HashMap::new(),
            },
        }
    }

    /// The topic this message belongs to.
    #[must_use]
    pub fn topic(&self) -> String {
        topic_name(&self.service, &self.action)
    }
}

/// Errors from the messaging layer.
#[derive(Debug, thiserror::Error)]
pub enum BusError {
    #[error("messaging channel closed")]
    Closed,
}

/// The messaging seam used by services and modules (TR-06-007).
#[async_trait]
pub trait MessageBus: Send + Sync {
    /// Produce `message` to `topic` (delivered once per subscribed group).
    async fn publish(&self, topic: &str, message: Message) -> Result<(), BusError>;
    /// Get a consumer for `topic` in consumer-group `group`. Additional
    /// consumers in the same group share the topic's messages.
    fn consumer(&self, topic: &str, group: &str) -> Consumer;
}

/// A consumer bound to a `(topic, group)`. Cloning within a group yields
/// competing consumers (load balancing, no double-processing).
#[derive(Clone)]
pub struct Consumer {
    rx: async_channel::Receiver<Message>,
}

impl Consumer {
    /// Await the next message for this group.
    ///
    /// # Errors
    /// [`BusError::Closed`] once the bus is dropped.
    pub async fn recv(&self) -> Result<Message, BusError> {
        self.rx.recv().await.map_err(|_| BusError::Closed)
    }

    /// Non-blocking receive.
    #[must_use]
    pub fn try_recv(&self) -> Option<Message> {
        self.rx.try_recv().ok()
    }
}

/// In-memory bus modelling topic fan-out to groups + competitive intra-group
/// delivery.
#[derive(Default)]
pub struct InMemoryBus {
    // (topic, group) -> the group's shared MPMC channel.
    groups: Mutex<
        HashMap<
            (String, String),
            (
                async_channel::Sender<Message>,
                async_channel::Receiver<Message>,
            ),
        >,
    >,
}

impl InMemoryBus {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl MessageBus for InMemoryBus {
    async fn publish(&self, topic: &str, message: Message) -> Result<(), BusError> {
        // Deliver a copy to each registered group for this topic (fan-out
        // across groups; one instance per group receives it).
        let senders: Vec<_> = {
            let groups = self.groups.lock().unwrap();
            groups
                .iter()
                .filter(|((t, _), _)| t == topic)
                .map(|(_, (tx, _))| tx.clone())
                .collect()
        };
        for tx in senders {
            let _ = tx.send(message.clone()).await;
        }
        Ok(())
    }

    fn consumer(&self, topic: &str, group: &str) -> Consumer {
        let mut groups = self.groups.lock().unwrap();
        let entry = groups
            .entry((topic.to_string(), group.to_string()))
            .or_insert_with(async_channel::unbounded);
        Consumer {
            rx: entry.1.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn topic_and_dlq_naming() {
        assert_eq!(topic_name("user", "created"), "superapp.user.created");
        assert_eq!(
            dlq_topic("superapp.user.created"),
            "superapp.user.created.dlq"
        );
    }

    #[test]
    fn message_envelope_has_required_schema() {
        let m = Message::new("user", "created", json!({"email":"a@b.com"}));
        let v = serde_json::to_value(&m).unwrap();
        assert!(v["id"].is_string());
        assert_eq!(v["service"], json!("user"));
        assert_eq!(v["action"], json!("created"));
        assert!(v["payload"].is_object());
        assert!(v["metadata"]["timestamp"].is_string());
        assert_eq!(v["metadata"]["attempt"], json!(1));
        assert_eq!(m.topic(), "superapp.user.created");
    }

    #[tokio::test]
    async fn single_consumer_receives_published_message() {
        let bus = InMemoryBus::new();
        let topic = topic_name("user", "created");
        let consumer = bus.consumer(&topic, "notifier");
        bus.publish(&topic, Message::new("user", "created", json!({"id":1})))
            .await
            .unwrap();
        let got = consumer.recv().await.unwrap();
        assert_eq!(got.action, "created");
    }

    #[tokio::test]
    async fn same_group_shares_without_double_processing() {
        let bus = InMemoryBus::new();
        let topic = topic_name("orders", "placed");
        // Two instances of the same service (same group).
        let c1 = bus.consumer(&topic, "orders-svc");
        let c2 = c1.clone(); // second instance shares the group channel
        for i in 0..4 {
            bus.publish(&topic, Message::new("orders", "placed", json!({ "n": i })))
                .await
                .unwrap();
        }
        // Drain competitively; union is all 4, with no message seen twice.
        let mut seen = std::collections::HashSet::new();
        for _ in 0..4 {
            let m = tokio::select! {
                r = c1.recv() => r.unwrap(),
                r = c2.recv() => r.unwrap(),
            };
            assert!(
                seen.insert(m.payload["n"].as_i64().unwrap()),
                "no double-processing"
            );
        }
        assert_eq!(seen.len(), 4);
    }

    #[tokio::test]
    async fn different_groups_each_receive_all() {
        let bus = InMemoryBus::new();
        let topic = topic_name("config", "updated");
        let audit = bus.consumer(&topic, "audit-svc");
        let cache = bus.consumer(&topic, "cache-svc");
        bus.publish(&topic, Message::new("config", "updated", json!({})))
            .await
            .unwrap();
        // Both groups get their own copy.
        assert_eq!(audit.recv().await.unwrap().action, "updated");
        assert_eq!(cache.recv().await.unwrap().action, "updated");
    }
}
