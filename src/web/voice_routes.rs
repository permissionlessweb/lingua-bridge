//! Voice channel web routes.
//!
//! Public URLs for viewing real-time voice translations.
//! Format: /voice/{guild_id}/{channel_id}

use crate::config::AppConfig;
use crate::web::broadcast::BroadcastManager;
use askama::Template;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::{Html, IntoResponse, Response},
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

/// Application state for voice routes
#[derive(Clone)]
pub struct VoiceAppState {
    pub broadcast: Arc<BroadcastManager>,
}

/// Askama template for the voice view
#[derive(Template)]
#[template(path = "voice_view.html")]
struct VoiceViewTemplate {
    guild_id: String,
    channel_id: String,
    ws_url: String,
}

/// Serve the voice channel web view
pub async fn voice_view(Path((guild_id, channel_id)): Path<(String, String)>) -> Response {
    let config = AppConfig::get();
    let ws_url = config
        .web
        .public_url
        .replace("http://", "ws://")
        .replace("https://", "wss://");

    let template = VoiceViewTemplate {
        guild_id,
        channel_id,
        ws_url,
    };
    Html(template.render().unwrap_or_default()).into_response()
}

/// WebSocket handler for voice channel updates
pub async fn voice_ws_handler(
    ws: WebSocketUpgrade,
    Path((guild_id, channel_id)): Path<(String, String)>,
    State(state): State<VoiceAppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_voice_socket(socket, guild_id, channel_id, state))
}

/// Handle a voice channel WebSocket connection
async fn handle_voice_socket(
    socket: WebSocket,
    guild_id: String,
    channel_id: String,
    state: VoiceAppState,
) {
    info!(guild_id, channel_id, "Voice WebSocket client connected");

    let (mut sender, mut receiver) = socket.split();

    // Subscribe to voice channel transcriptions
    let mut broadcast_rx = state
        .broadcast
        .subscribe_voice_channel(&guild_id, &channel_id);

    // Send welcome message
    let welcome = serde_json::json!({
        "type": "welcome",
        "guild_id": guild_id,
        "channel_id": channel_id,
        "message": "Connected to voice channel transcription feed"
    });
    if let Err(e) = sender.send(Message::Text(welcome.to_string().into())).await {
        error!(error = %e, "Failed to send welcome message");
        return;
    }

    // Ping interval for keepalive
    let mut ping_interval = interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            // Forward broadcast messages to client
            result = broadcast_rx.recv() => {
                match result {
                    Ok(msg) => {
                        match serde_json::to_string(&msg) {
                            Ok(json) => {
                                if let Err(e) = sender.send(Message::Text(json.into())).await {
                                    debug!(error = %e, "Failed to send message, client disconnected");
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, "Failed to serialize message");
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!(skipped = n, "Client lagged, skipped messages");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        debug!("Broadcast channel closed");
                        break;
                    }
                }
            }

            // Handle incoming messages from client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Handle ping/pong or other client messages
                        if text.as_str() == "ping" {
                            let _ = sender.send(Message::Text("pong".into())).await;
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = sender.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) => {
                        debug!("Client closed connection");
                        break;
                    }
                    Some(Err(e)) => {
                        debug!(error = %e, "WebSocket error");
                        break;
                    }
                    None => {
                        debug!("WebSocket stream ended");
                        break;
                    }
                    _ => {}
                }
            }

            // Send periodic ping
            _ = ping_interval.tick() => {
                if let Err(e) = sender.send(Message::Ping(vec![].into())).await {
                    debug!(error = %e, "Failed to send ping");
                    break;
                }
            }
        }
    }

    info!(guild_id, channel_id, "Voice WebSocket client disconnected");
}

