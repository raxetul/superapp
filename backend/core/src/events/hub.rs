//! In-process real-time event hub (TR-06-002/003/008).
//!
//! Publishers push [`EventEnvelope`]s; subscribers each receive the broadcasts
//! plus events targeted at their user. Every event gets a monotonic sequence
//! id; a bounded ring buffer retains recent events so a reconnecting client can
//! **resume** from its `Last-Event-ID` within the window (TR-06-008).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use tokio::sync::broadcast;

use super::envelope::EventEnvelope;

const DEFAULT_BUFFER: usize = 256;
const DEFAULT_CHANNEL: usize = 1024;

/// An event plus its monotonic sequence id (the SSE event id).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequencedEvent {
    pub id: u64,
    pub event: EventEnvelope,
}

/// Fan-out hub for real-time events.
pub struct EventHub {
    tx: broadcast::Sender<SequencedEvent>,
    seq: AtomicU64,
    buffer: Mutex<VecDeque<SequencedEvent>>,
    buffer_cap: usize,
}

impl Default for EventHub {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_BUFFER, DEFAULT_CHANNEL)
    }
}

impl EventHub {
    /// A hub retaining `buffer_cap` events for resume, with a `channel_cap`
    /// per-subscriber backlog.
    #[must_use]
    pub fn with_capacity(buffer_cap: usize, channel_cap: usize) -> Self {
        let (tx, _rx) = broadcast::channel(channel_cap);
        Self {
            tx,
            seq: AtomicU64::new(0),
            buffer: Mutex::new(VecDeque::with_capacity(buffer_cap)),
            buffer_cap,
        }
    }

    /// Publish `event`, returning its sequence id. Delivery to zero subscribers
    /// is not an error.
    pub fn publish(&self, event: EventEnvelope) -> u64 {
        let id = self.seq.fetch_add(1, Ordering::SeqCst) + 1;
        let seq_ev = SequencedEvent { id, event };
        {
            let mut buf = self.buffer.lock().unwrap();
            if buf.len() == self.buffer_cap {
                buf.pop_front();
            }
            buf.push_back(seq_ev.clone());
        }
        let _ = self.tx.send(seq_ev);
        id
    }

    /// Subscribe as `user_id`; the subscription yields broadcasts and events
    /// targeted at that user.
    #[must_use]
    pub fn subscribe(&self, user_id: impl Into<String>) -> Subscription {
        Subscription {
            rx: self.tx.subscribe(),
            user: user_id.into(),
        }
    }

    /// Buffered events with id greater than `last_id` that are relevant to
    /// `user_id` — the resume window (TR-06-008).
    #[must_use]
    pub fn replay_since(&self, last_id: u64, user_id: &str) -> Vec<SequencedEvent> {
        self.buffer
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.id > last_id && s.event.is_for(user_id))
            .cloned()
            .collect()
    }
}

/// A per-subscriber handle that filters events for its user.
pub struct Subscription {
    rx: broadcast::Receiver<SequencedEvent>,
    user: String,
}

impl Subscription {
    /// Await the next event relevant to this subscriber. `None` once the hub is
    /// dropped. Lagged (buffer-overflow) notifications are skipped.
    pub async fn recv(&mut self) -> Option<SequencedEvent> {
        loop {
            match self.rx.recv().await {
                Ok(seq) if seq.event.is_for(&self.user) => return Some(seq),
                Ok(_) => continue,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn broadcast_reaches_all_subscribers() {
        let hub = EventHub::default();
        let mut a = hub.subscribe("alice@x");
        let mut b = hub.subscribe("bob@x");
        hub.publish(EventEnvelope::broadcast("config.updated", json!({"k":1})));
        assert_eq!(a.recv().await.unwrap().event.type_, "config.updated");
        assert_eq!(b.recv().await.unwrap().event.type_, "config.updated");
    }

    #[tokio::test]
    async fn targeted_event_reaches_only_target() {
        let hub = EventHub::default();
        let mut a = hub.subscribe("alice@x");
        let mut b = hub.subscribe("bob@x");
        hub.publish(EventEnvelope::targeted(
            "user.created",
            json!({}),
            "alice@x",
        ));
        hub.publish(EventEnvelope::broadcast("config.updated", json!({})));

        // Alice gets the targeted event first, then the broadcast.
        assert_eq!(a.recv().await.unwrap().event.type_, "user.created");
        assert_eq!(a.recv().await.unwrap().event.type_, "config.updated");

        // Bob skips the alice-targeted event; his first event is the broadcast.
        let first = timeout(Duration::from_secs(1), b.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first.event.type_, "config.updated");
    }

    #[tokio::test]
    async fn replay_since_returns_missed_events_for_resume() {
        let hub = EventHub::default();
        let id1 = hub.publish(EventEnvelope::broadcast("a", json!({})));
        let _id2 = hub.publish(EventEnvelope::targeted("b", json!({}), "bob@x"));
        let id3 = hub.publish(EventEnvelope::broadcast("c", json!({})));

        // Resuming after id1 as alice: she missed the broadcast c (not b, which
        // targeted bob).
        let missed = hub.replay_since(id1, "alice@x");
        let ids: Vec<u64> = missed.iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![id3]);

        // Bob resuming after id1 gets both b and c.
        assert_eq!(hub.replay_since(id1, "bob@x").len(), 2);
    }
}
