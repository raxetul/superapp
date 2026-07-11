//! Real-time event stream endpoint (TR-06-001 / TR-06-008).
//!
//! `GET /api/v1/events/stream` is an authenticated Server-Sent Events stream.
//! The client receives broadcasts and events targeted at its user. On
//! reconnect it may send `Last-Event-ID`; the server replays buffered events
//! after that id (resume within the bounded window) before resuming live
//! delivery.

use std::collections::VecDeque;
use std::convert::Infallible;
use std::sync::Arc;

use axum::http::HeaderMap;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Extension;
use futures::Stream;
use loco_rs::prelude::*;

use crate::auth::extractor::AuthedClaims;
use crate::auth::state::AuthState;
use crate::events::hub::SequencedEvent;

/// `GET /events/stream` — authenticated SSE stream.
async fn stream(
    AuthedClaims(claims): AuthedClaims,
    Extension(state): Extension<Arc<AuthState>>,
    headers: HeaderMap,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let user = claims.email().unwrap_or_default().to_string();

    // Resume: replay buffered events after Last-Event-ID (TR-06-008).
    let last_id = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());
    let missed: VecDeque<SequencedEvent> = match last_id {
        Some(id) => state.events.replay_since(id, &user).into(),
        None => VecDeque::new(),
    };

    let subscription = state.events.subscribe(&user);

    let stream =
        futures::stream::unfold((missed, subscription), |(mut missed, mut sub)| async move {
            let next = if let Some(m) = missed.pop_front() {
                Some(m)
            } else {
                sub.recv().await
            };
            next.map(|seq| {
                let event = Event::default()
                    .id(seq.id.to_string())
                    .event(seq.event.type_.clone())
                    .json_data(&seq.event)
                    .unwrap_or_else(|_| Event::default().comment("serialization error"));
                (Ok::<Event, Infallible>(event), (missed, sub))
            })
        });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Routes mounted under the versioned API base.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/events")
        .add("/stream", get(stream))
}
