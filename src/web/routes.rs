use crate::config::AppConfig;
use crate::db::{DbPool, WebSessionRepo};
use crate::translation::TranslationClient;
use crate::web::websocket::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::info;

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Health check endpoint
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Session info response
#[derive(Serialize)]
pub struct SessionInfo {
    pub valid: bool,
    pub guild_id: Option<String>,
    pub channel_id: Option<String>,
    pub expires_at: Option<String>,
}

/// Get session info
pub async fn get_session_info(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> Json<SessionInfo> {
    match WebSessionRepo::get_by_session_id(&state.pool, &session_id).await {
        Ok(Some(session)) => Json(SessionInfo {
            valid: true,
            guild_id: Some(session.guild_id),
            channel_id: session.channel_id,
            expires_at: Some(session.expires_at.to_rfc3339()),
        }),
        _ => Json(SessionInfo {
            valid: false,
            guild_id: None,
            channel_id: None,
            expires_at: None,
        }),
    }
}

/// Translation cache stats
pub async fn cache_stats(
    State(translator): State<Arc<TranslationClient>>,
) -> Json<crate::translation::CacheStats> {
    Json(translator.cache_stats())
}

/// Serve the web view HTML
pub async fn web_view(Path(session_id): Path<String>) -> Html<String> {
    Html(generate_web_view_html(&session_id))
}

/// Generate the web view HTML
fn generate_web_view_html(session_id: &str) -> String {
    let config = AppConfig::get();
    let ws_url = config
        .web
        .public_url
        .replace("http://", "ws://")
        .replace("https://", "wss://");

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>LinguaBridge - Live Translations</title>
    <style>
        :root {{
            --bg-primary: #36393f;
            --bg-secondary: #2f3136;
            --bg-tertiary: #202225;
            --text-primary: #dcddde;
            --text-secondary: #8e9297;
            --accent: #5865f2;
            --success: #3ba55d;
        }}
        * {{
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }}
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: var(--bg-primary);
            color: var(--text-primary);
            height: 100vh;
            display: flex;
            flex-direction: column;
        }}
        header {{
            background: var(--bg-tertiary);
            padding: 1rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
            border-bottom: 1px solid rgba(255,255,255,0.1);
        }}
        header h1 {{
            font-size: 1.25rem;
            font-weight: 600;
        }}
        .status {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
            font-size: 0.875rem;
            color: var(--text-secondary);
        }}
        .status-dot {{
            width: 8px;
            height: 8px;
            border-radius: 50%;
            background: #ed4245;
        }}
        .status-dot.connected {{
            background: var(--success);
        }}
        #messages {{
            flex: 1;
            overflow-y: auto;
            padding: 1rem;
        }}
        .message {{
            background: var(--bg-secondary);
            border-radius: 8px;
            padding: 1rem;
            margin-bottom: 0.75rem;
            animation: slideIn 0.3s ease;
        }}
        @keyframes slideIn {{
            from {{
                opacity: 0;
                transform: translateY(10px);
            }}
            to {{
                opacity: 1;
                transform: translateY(0);
            }}
        }}
        .message-header {{
            display: flex;
            justify-content: space-between;
            margin-bottom: 0.5rem;
        }}
        .author {{
            font-weight: 600;
            color: var(--accent);
        }}
        .timestamp {{
            font-size: 0.75rem;
            color: var(--text-secondary);
        }}
        .original {{
            color: var(--text-secondary);
            font-size: 0.875rem;
            margin-bottom: 0.5rem;
            padding-left: 0.75rem;
            border-left: 2px solid var(--text-secondary);
        }}
        .translated {{
            font-size: 1rem;
        }}
        .lang-badge {{
            display: inline-block;
            background: var(--accent);
            color: white;
            font-size: 0.625rem;
            padding: 0.125rem 0.375rem;
            border-radius: 4px;
            text-transform: uppercase;
            margin-left: 0.5rem;
        }}
        .empty-state {{
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            height: 100%;
            color: var(--text-secondary);
        }}
        .empty-state svg {{
            width: 64px;
            height: 64px;
            margin-bottom: 1rem;
            opacity: 0.5;
        }}
    </style>
</head>
<body>
    <header>
        <h1>LinguaBridge</h1>
        <div class="status">
            <div class="status-dot" id="statusDot"></div>
            <span id="statusText">Connecting...</span>
        </div>
    </header>
    <div id="messages">
        <div class="empty-state" id="emptyState">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
            </svg>
            <p>Waiting for messages...</p>
        </div>
    </div>
    <script>
        const sessionId = '{}';
        const wsUrl = '{}/ws/' + sessionId;
        const messagesEl = document.getElementById('messages');
        const emptyState = document.getElementById('emptyState');
        const statusDot = document.getElementById('statusDot');
        const statusText = document.getElementById('statusText');

        let ws;
        let reconnectAttempts = 0;

        function connect() {{
            ws = new WebSocket(wsUrl);

            ws.onopen = () => {{
                statusDot.classList.add('connected');
                statusText.textContent = 'Connected';
                reconnectAttempts = 0;
            }};

            ws.onclose = () => {{
                statusDot.classList.remove('connected');
                statusText.textContent = 'Disconnected';

                // Reconnect with exponential backoff
                const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), 30000);
                reconnectAttempts++;
                setTimeout(connect, delay);
            }};

            ws.onerror = (error) => {{
                console.error('WebSocket error:', error);
            }};

            ws.onmessage = (event) => {{
                const data = JSON.parse(event.data);

                if (data.type === 'translation') {{
                    addMessage(data);
                }} else if (data.type === 'error') {{
                    statusText.textContent = data.message;
                }}
            }};
        }}

        function addMessage(data) {{
            emptyState.style.display = 'none';

            const messageEl = document.createElement('div');
            messageEl.className = 'message';

            const time = new Date(data.timestamp).toLocaleTimeString();

            messageEl.innerHTML = `
                <div class="message-header">
                    <span class="author">${{escapeHtml(data.author_name)}}</span>
                    <span class="timestamp">${{time}}</span>
                </div>
                <div class="original">${{escapeHtml(data.original_text)}}</div>
                <div class="translated">
                    ${{escapeHtml(data.translated_text)}}
                    <span class="lang-badge">${{data.source_lang}} → ${{data.target_lang}}</span>
                </div>
            `;

            messagesEl.appendChild(messageEl);
            messagesEl.scrollTop = messagesEl.scrollHeight;

            // Limit messages in DOM
            while (messagesEl.children.length > 100) {{
                messagesEl.removeChild(messagesEl.children[1]);
            }}
        }}

        function escapeHtml(text) {{
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }}

        connect();
    </script>
</body>
</html>"#,
        session_id, ws_url
    )
}

/// Create the web router
pub fn create_router(state: AppState, translator: Arc<TranslationClient>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health))
        .route("/view/{session_id}", get(web_view))
        .route("/ws/{session_id}", get(crate::web::websocket::ws_handler))
        .route("/api/session/{session_id}", get(get_session_info))
        .with_state(state)
        .route(
            "/api/cache/stats",
            get(cache_stats).with_state(translator),
        )
        .nest_service("/static", ServeDir::new("static"))
        .layer(cors)
}
