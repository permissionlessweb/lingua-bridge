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
