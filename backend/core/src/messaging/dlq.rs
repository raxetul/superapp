//! Retry + dead-letter queue (TR-06-006).
//!
//! A message that keeps failing is retried up to a bound and then routed to a
//! dead-letter topic (`{topic}.dlq`), where the entry preserves the **original
//! message** plus failure metadata (attempts + last error).

use std::future::Future;

use super::{dlq_topic, BusError, Message, MessageBus};

/// How many times to attempt processing before dead-lettering.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_attempts: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self { max_attempts: 3 }
    }
}

/// A message that exhausted its retries.
#[derive(Debug, Clone)]
pub struct DeadLetter {
    pub original: Message,
    pub attempts: u32,
    pub error: String,
}

/// Attempt to process `message` with `handler`, retrying up to
/// `policy.max_attempts`. Returns `Ok` on success, or a [`DeadLetter`] when all
/// attempts fail. The per-attempt count is reflected in `metadata.attempt`.
///
/// # Errors
/// [`DeadLetter`] when every attempt fails.
pub async fn process_with_retries<H, Fut>(
    message: Message,
    policy: RetryPolicy,
    mut handler: H,
) -> Result<(), DeadLetter>
where
    H: FnMut(Message) -> Fut,
    Fut: Future<Output = Result<(), String>>,
{
    let mut last_error = String::new();
    for attempt in 1..=policy.max_attempts {
        let mut attempt_msg = message.clone();
        attempt_msg.metadata.attempt = attempt;
        match handler(attempt_msg).await {
            Ok(()) => return Ok(()),
            Err(e) => last_error = e,
        }
    }
    Err(DeadLetter {
        original: message,
        attempts: policy.max_attempts,
        error: last_error,
    })
}

/// Route a [`DeadLetter`] to the dead-letter topic derived from `topic`,
/// preserving the original message and failure metadata.
///
/// # Errors
/// [`BusError`] if publishing fails.
pub async fn route_to_dlq(
    bus: &dyn MessageBus,
    topic: &str,
    dead: &DeadLetter,
) -> Result<(), BusError> {
    let mut msg = Message::new(
        &dead.original.service,
        &format!("{}.dlq", dead.original.action),
        serde_json::json!({
            "original": dead.original,
            "error": dead.error,
            "attempts": dead.attempts,
        }),
    );
    msg.metadata
        .headers
        .insert("x-dlq-reason".to_string(), dead.error.clone());
    msg.metadata
        .headers
        .insert("x-dlq-attempts".to_string(), dead.attempts.to_string());
    bus.publish(&dlq_topic(topic), msg).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::{topic_name, InMemoryBus};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn failing_message_is_retried_then_dead_lettered_preserving_original() {
        let bus = InMemoryBus::new();
        let topic = topic_name("orders", "placed");
        let dlq = bus.consumer(&dlq_topic(&topic), "dlq-monitor");

        let original = Message::new("orders", "placed", serde_json::json!({"order": 7}));
        let attempts_seen = Arc::new(AtomicU32::new(0));
        let seen = attempts_seen.clone();

        // Handler always fails.
        let result = process_with_retries(original.clone(), RetryPolicy { max_attempts: 3 }, |m| {
            let seen = seen.clone();
            async move {
                seen.fetch_add(1, Ordering::SeqCst);
                Err(format!("boom on attempt {}", m.metadata.attempt))
            }
        })
        .await;

        let dead = result.expect_err("should dead-letter after retries");
        assert_eq!(dead.attempts, 3);
        assert_eq!(attempts_seen.load(Ordering::SeqCst), 3, "retried 3 times");

        route_to_dlq(&bus, &topic, &dead).await.unwrap();

        // The DLQ entry preserves the original message + failure metadata.
        let dl_msg = dlq.recv().await.unwrap();
        assert_eq!(
            dl_msg.payload["original"]["payload"]["order"],
            serde_json::json!(7)
        );
        assert_eq!(dl_msg.payload["attempts"], serde_json::json!(3));
        assert_eq!(dl_msg.metadata.headers.get("x-dlq-attempts").unwrap(), "3");
        assert!(dl_msg.metadata.headers.contains_key("x-dlq-reason"));
    }

    #[tokio::test]
    async fn message_succeeding_within_retries_is_not_dead_lettered() {
        let fail_until = Arc::new(AtomicU32::new(2)); // fail first 2 attempts
        let fu = fail_until.clone();
        let result = process_with_retries(
            Message::new("orders", "placed", serde_json::json!({})),
            RetryPolicy { max_attempts: 5 },
            move |_m| {
                let fu = fu.clone();
                async move {
                    if fu.fetch_sub(1, Ordering::SeqCst) > 0 {
                        Err("transient".to_string())
                    } else {
                        Ok(())
                    }
                }
            },
        )
        .await;
        assert!(result.is_ok(), "should succeed on the 3rd attempt");
    }
}
