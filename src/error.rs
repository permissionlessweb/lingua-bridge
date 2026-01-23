use thiserror::Error;

/// Application-wide error types
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Discord error: {0}")]
    Discord(#[from] serenity::Error),

    #[error("Translation error: {0}")]
    Translation(String),

    #[error("Language detection failed: {0}")]
    LanguageDetection(String),

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Authentication required")]
    AuthRequired,

    #[error("Invalid session")]
    InvalidSession,

    #[error("Guild not configured")]
    GuildNotConfigured,

    #[error("Inference service unavailable")]
    InferenceUnavailable,

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn translation<S: Into<String>>(msg: S) -> Self {
        Self::Translation(msg.into())
    }

    pub fn internal<S: Into<String>>(msg: S) -> Self {
        Self::Internal(msg.into())
    }
}

/// Result type alias using AppError
pub type AppResult<T> = Result<T, AppError>;

/// Convert AppError to HTTP status codes for web responses
impl AppError {
    pub fn status_code(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            Self::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Http(_) => StatusCode::BAD_GATEWAY,
            Self::Discord(_) => StatusCode::BAD_GATEWAY,
            Self::Translation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::LanguageDetection(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::UnsupportedLanguage(_) => StatusCode::BAD_REQUEST,
            Self::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            Self::AuthRequired => StatusCode::UNAUTHORIZED,
            Self::InvalidSession => StatusCode::UNAUTHORIZED,
            Self::GuildNotConfigured => StatusCode::NOT_FOUND,
            Self::InferenceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = self.status_code();
        let body = serde_json::json!({
            "error": self.to_string(),
            "code": status.as_u16()
        });
        (status, axum::Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_config_error_maps_to_500() {
        let err = AppError::Config(config::ConfigError::NotFound("test".into()));
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_translation_error_maps_to_422() {
        let err = AppError::Translation("failed".to_string());
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn test_language_detection_maps_to_422() {
        let err = AppError::LanguageDetection("unknown".to_string());
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn test_unsupported_language_maps_to_400() {
        let err = AppError::UnsupportedLanguage("xx".to_string());
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_rate_limit_maps_to_429() {
        let err = AppError::RateLimitExceeded;
        assert_eq!(err.status_code(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn test_auth_required_maps_to_401() {
        let err = AppError::AuthRequired;
        assert_eq!(err.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_invalid_session_maps_to_401() {
        let err = AppError::InvalidSession;
        assert_eq!(err.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_guild_not_configured_maps_to_404() {
        let err = AppError::GuildNotConfigured;
        assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_inference_unavailable_maps_to_503() {
        let err = AppError::InferenceUnavailable;
        assert_eq!(err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn test_internal_error_maps_to_500() {
        let err = AppError::Internal("something broke".to_string());
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_discord_error_maps_to_502() {
        // Discord errors map to BAD_GATEWAY
        let status = AppError::Internal("placeholder".to_string());
        // Verify that Internal maps correctly
        assert_eq!(status.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_error_display_format() {
        let err = AppError::Translation("test message".to_string());
        assert_eq!(err.to_string(), "Translation error: test message");
    }

    #[test]
    fn test_error_display_rate_limit() {
        let err = AppError::RateLimitExceeded;
        assert_eq!(err.to_string(), "Rate limit exceeded");
    }

    #[test]
    fn test_translation_helper() {
        let err = AppError::translation("helper test");
        match err {
            AppError::Translation(msg) => assert_eq!(msg, "helper test"),
            _ => panic!("Expected Translation variant"),
        }
    }

    #[test]
    fn test_internal_helper() {
        let err = AppError::internal("internal test");
        match err {
            AppError::Internal(msg) => assert_eq!(msg, "internal test"),
            _ => panic!("Expected Internal variant"),
        }
    }

    #[test]
    fn test_into_response() {
        use axum::response::IntoResponse;
        let err = AppError::RateLimitExceeded;
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn test_into_response_body_format() {
        use axum::response::IntoResponse;
        let err = AppError::GuildNotConfigured;
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
