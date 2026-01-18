use crate::db::{DbPool, WebSessionRepo};
use crate::web::broadcast::BroadcastManager;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast::error::RecvError;
use tracing::{debug, error, info, warn};

/// Application state for web handlers
#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub broadcast: Arc<BroadcastManager>,
}

/// WebSocket upgrade handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, session_id, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, session_id: String, state: AppState) {
    // Validate session
    let session = match WebSessionRepo::get_by_session_id(&state.pool, &session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            warn!(
                "Invalid session attempted: {}",
                &session_id[..8.min(session_id.len())]
            );
            let (mut sender, _) = socket.split();
            let _ = sender
                .send(Message::Text(
                    serde_json::json!({"type": "error", "message": "Invalid or expired session"})
                        .to_string()
                        .into(),
                ))
                .await;
            return;
        }
        Err(e) => {
            error!("Session lookup failed: {}", e);
            return;
        }
    };

    info!(
        "WebSocket connected: session={}, guild={}, channel={:?}",
        &session.session_id[..8],
        session.guild_id,
        session.channel_id
    );

    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast
    let mut rx = if let Some(ref channel_id) = session.channel_id {
        state.broadcast.subscribe_channel(channel_id)
    } else {
        state.broadcast.subscribe_global()
    };

    // Send welcome message
    let welcome = serde_json::json!({
        "type": "connected",
        "guild_id": session.guild_id,
        "channel_id": session.channel_id,
    });
    if sender
        .send(Message::Text(welcome.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    // Spawn task to receive broadcast messages and forward to client
    let send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    let json = match serde_json::to_string(&msg) {
                        Ok(j) => j,
                        Err(e) => {
                            error!("Failed to serialize message: {}", e);
                            continue;
                        }
                    };
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                Err(RecvError::Lagged(n)) => {
                    warn!("WebSocket lagged {} messages", n);
                    continue;
                }
                Err(RecvError::Closed) => {
                    break;
                }
            }
        }
    });

    // Spawn task to receive client messages (heartbeats, etc.)
    let recv_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            match result {
                Ok(Message::Text(text)) => {
                    debug!("Received from client: {}", text);
                    // Handle client messages if needed (e.g., heartbeat, language filter)
                }
                Ok(Message::Ping(_)) => {
                    debug!("Received ping");
                }
                Ok(Message::Pong(_)) => {
                    debug!("Received pong");
                }
                Ok(Message::Close(_)) => {
                    info!("Client closed connection");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {
            debug!("Send task completed");
        }
        _ = recv_task => {
            debug!("Receive task completed");
        }
    }

    info!(
        "WebSocket disconnected: session={}",
        &session.session_id[..8]
    );
}
