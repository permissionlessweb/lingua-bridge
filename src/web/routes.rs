use crate::config::AppConfig;
use crate::db::WebSessionRepo;
use crate::translation::TranslationClient;
use crate::web::voice_routes::{voice_view, voice_ws_handler, VoiceAppState};
use crate::web::websocket::AppState;
use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Json, Response},
    routing::get,
    Router,
};
use serde::Serialize;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

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

/// Askama template for the web view
#[derive(Template)]
#[template(path = "web_view.html")]
struct WebViewTemplate {
    session_id: String,
    ws_url: String,
}

/// Serve the web view HTML
pub async fn web_view(Path(session_id): Path<String>) -> Response {
    let config = AppConfig::get();
    let ws_url = config
        .web
        .public_url
        .replace("http://", "ws://")
        .replace("https://", "wss://");

    let template = WebViewTemplate { session_id, ws_url };
    Html(template.render().unwrap_or_default()).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::broadcast::BroadcastManager;
    use crate::db::queries::setup_test_db;

    #[tokio::test]
    async fn test_health_returns_ok() {
        let resp = health().await;
        assert_eq!(resp.0.status, "ok");
        assert!(!resp.0.version.is_empty());
    }

    #[tokio::test]
    async fn test_health_version_matches_cargo() {
        let resp = health().await;
        assert_eq!(resp.0.version, env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn test_get_session_info_valid() {
        let pool = setup_test_db().await;
        let broadcast = Arc::new(BroadcastManager::new());
        let state = AppState {
            pool: pool.clone(),
            broadcast,
        };

        // Create a session first
        let new_session = crate::db::models::NewWebSession {
            user_id: "u1".to_string(),
            guild_id: "g1".to_string(),
            channel_id: Some("ch1".to_string()),
        };
        let session = crate::db::WebSessionRepo::create(&pool, new_session, 24)
            .await
            .unwrap();

        // Query session info
        let resp = get_session_info(
            Path(session.session_id),
            State(state),
        )
        .await;

        assert!(resp.0.valid);
        assert_eq!(resp.0.guild_id, Some("g1".to_string()));
        assert_eq!(resp.0.channel_id, Some("ch1".to_string()));
        assert!(resp.0.expires_at.is_some());
    }

    #[tokio::test]
    async fn test_get_session_info_invalid() {
        let pool = setup_test_db().await;
        let broadcast = Arc::new(BroadcastManager::new());
        let state = AppState {
            pool,
            broadcast,
        };

        let resp = get_session_info(
            Path("nonexistent-session".to_string()),
            State(state),
        )
        .await;

        assert!(!resp.0.valid);
        assert!(resp.0.guild_id.is_none());
        assert!(resp.0.channel_id.is_none());
        assert!(resp.0.expires_at.is_none());
    }

    #[test]
    fn test_health_response_serialize() {
        let resp = HealthResponse {
            status: "ok".to_string(),
            version: "0.1.0".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"version\":\"0.1.0\""));
    }

    #[test]
    fn test_session_info_serialize_valid() {
        let info = SessionInfo {
            valid: true,
            guild_id: Some("g123".to_string()),
            channel_id: Some("ch456".to_string()),
            expires_at: Some("2025-01-01T00:00:00Z".to_string()),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"valid\":true"));
        assert!(json.contains("g123"));
    }

    #[test]
    fn test_session_info_serialize_invalid() {
        let info = SessionInfo {
            valid: false,
            guild_id: None,
            channel_id: None,
            expires_at: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"valid\":false"));
        assert!(json.contains("null"));
    }
}

/// Create the web router
pub fn create_router(state: AppState, translator: Arc<TranslationClient>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Voice routes state
    let voice_state = VoiceAppState {
        broadcast: state.broadcast.clone(),
    };

    Router::new()
        .route("/health", get(health))
        // Text channel translation routes (session-based)
        .route("/view/{session_id}", get(web_view))
        .route("/ws/{session_id}", get(crate::web::websocket::ws_handler))
        .route("/api/session/{session_id}", get(get_session_info))
        .with_state(state)
        // Voice channel routes (public)
        .route("/voice/{guild_id}/{channel_id}", get(voice_view))
        .route(
            "/voice/{guild_id}/{channel_id}/ws",
            get(voice_ws_handler).with_state(voice_state),
        )
        .route(
            "/api/cache/stats",
            get(cache_stats).with_state(translator),
        )
        .nest_service("/static", ServeDir::new("static"))
        .layer(cors)
}
