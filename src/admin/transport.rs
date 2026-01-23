//! Admin transport layer - HTTP endpoints for secure provisioning.
//!
//! Exposes endpoints for:
//! - Getting the bot's ephemeral public key
//! - Checking provisioning status
//! - Receiving encrypted secrets from admin

use crate::admin::crypto::{
    build_signature_message, decrypt_payload, parse_ed25519_public_key, parse_signature,
    parse_x25519_public_key, verify_signature, CryptoError, EphemeralKeyPair,
};
use crate::admin::secrets::{ProvisioningStatus, SecretsPayload, SharedSecretStore};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Provisioning request from admin CLI.
#[derive(Debug, Deserialize)]
pub struct ProvisionRequest {
    /// Admin's ephemeral X25519 public key (base64)
    pub admin_x25519_public: String,
    /// Encrypted secrets payload (base64)
    pub ciphertext: String,
    /// ChaCha20-Poly1305 nonce (base64)
    pub nonce: String,
    /// Ed25519 signature over (admin_x25519_public || ciphertext || nonce)
    pub signature: String,
}

/// Response for public key endpoint.
#[derive(Debug, Serialize)]
pub struct PublicKeyResponse {
    /// Bot's ephemeral X25519 public key (base64)
    pub public_key: String,
}

/// Response for status endpoint.
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: ProvisioningStatus,
}

/// Response for provision endpoint.
#[derive(Debug, Serialize)]
pub struct ProvisionResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Error response.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Admin transport errors.
#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    #[error("Cryptographic error: {0}")]
    Crypto(#[from] CryptoError),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Already provisioned")]
    AlreadyProvisioned,

    #[error("Secrets deserialization failed: {0}")]
    DeserializationFailed(String),
}

impl IntoResponse for AdminError {
    fn into_response(self) -> Response {
        let status = match &self {
            AdminError::Crypto(_) => StatusCode::BAD_REQUEST,
            AdminError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            AdminError::AlreadyProvisioned => StatusCode::CONFLICT,
            AdminError::DeserializationFailed(_) => StatusCode::BAD_REQUEST,
        };

        let body = Json(ErrorResponse {
            error: self.to_string(),
        });

        (status, body).into_response()
    }
}

/// Shared state for admin endpoints.
pub struct AdminState {
    /// Bot's ephemeral keypair
    pub keypair: RwLock<Option<EphemeralKeyPair>>,
    /// Admin's Ed25519 public key for signature verification
    pub admin_public_key: VerifyingKey,
    /// Secret store to provision
    pub secret_store: SharedSecretStore,
}

impl AdminState {
    /// Create new admin state.
    ///
    /// # Arguments
    /// * `admin_public_key_base64` - Admin's Ed25519 public key in base64
    /// * `secret_store` - Shared secret store
    pub fn new(
        admin_public_key_base64: &str,
        secret_store: SharedSecretStore,
    ) -> Result<Self, CryptoError> {
        let admin_public_key = parse_ed25519_public_key(admin_public_key_base64)?;

        // Generate ephemeral keypair
        let keypair = EphemeralKeyPair::generate();
        info!("Generated ephemeral X25519 keypair for admin provisioning");

        Ok(Self {
            keypair: RwLock::new(Some(keypair)),
            admin_public_key,
            secret_store,
        })
    }
}

/// Handler: GET /admin/pubkey
///
/// Returns the bot's ephemeral X25519 public key.
async fn get_public_key(State(state): State<Arc<AdminState>>) -> Result<Json<PublicKeyResponse>, AdminError> {
    let guard = state.keypair.read().await;
    let keypair = guard
        .as_ref()
        .ok_or_else(|| AdminError::AlreadyProvisioned)?;

    Ok(Json(PublicKeyResponse {
        public_key: keypair.public_key_base64(),
    }))
}

/// Handler: GET /admin/status
///
/// Returns current provisioning status.
async fn get_status(State(state): State<Arc<AdminState>>) -> Json<StatusResponse> {
    let status = state.secret_store.status().await;
    Json(StatusResponse { status })
}

/// Handler: POST /admin/provision
///
/// Receives encrypted secrets from admin, verifies signature, decrypts, and stores.
async fn provision(
    State(state): State<Arc<AdminState>>,
    Json(request): Json<ProvisionRequest>,
) -> Result<Json<ProvisionResponse>, AdminError> {
    // Check if already provisioned
    if state.secret_store.is_provisioned().await {
        warn!("Provision attempt when already provisioned");
        return Err(AdminError::AlreadyProvisioned);
    }

    // Take the keypair (consuming it, ensures single use)
    let keypair = {
        let mut guard = state.keypair.write().await;
        guard.take().ok_or(AdminError::AlreadyProvisioned)?
    };

    info!("Processing provision request...");

    // Parse admin's X25519 public key
    let admin_x25519_public = parse_x25519_public_key(&request.admin_x25519_public)?;

    // Decode ciphertext and nonce for signature verification
    let ciphertext_bytes = BASE64
        .decode(&request.ciphertext)
        .map_err(|e| AdminError::Crypto(CryptoError::Base64(e)))?;
    let nonce_bytes = BASE64
        .decode(&request.nonce)
        .map_err(|e| AdminError::Crypto(CryptoError::Base64(e)))?;

    // Build message that was signed
    let message = build_signature_message(
        admin_x25519_public.as_bytes(),
        &ciphertext_bytes,
        &nonce_bytes,
    );

    // Parse and verify signature
    let signature = parse_signature(&request.signature)?;
    verify_signature(&state.admin_public_key, &message, &signature)?;
    info!("Signature verified successfully");

    // Compute shared secret and decrypt
    let shared_secret = keypair.diffie_hellman(&admin_x25519_public);
    let plaintext = decrypt_payload(&shared_secret, &request.nonce, &request.ciphertext)?;
    info!("Decryption successful");

    // Parse secrets
    let secrets: SecretsPayload = serde_json::from_slice(&plaintext)
        .map_err(|e| AdminError::DeserializationFailed(e.to_string()))?;

    // Store secrets
    if !state.secret_store.provision(secrets).await {
        error!("Failed to store secrets - already provisioned");
        return Err(AdminError::AlreadyProvisioned);
    }

    info!("Secrets provisioned successfully!");

    Ok(Json(ProvisionResponse {
        success: true,
        message: Some("Secrets provisioned successfully".to_string()),
    }))
}

/// Create the admin router.
pub fn admin_router(state: Arc<AdminState>) -> Router {
    Router::new()
        .route("/pubkey", get(get_public_key))
        .route("/status", get(get_status))
        .route("/provision", post(provision))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin::secrets::create_secret_store;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use x25519_dalek::EphemeralSecret;

    fn generate_admin_keys() -> (SigningKey, String) {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let public_key_base64 = BASE64.encode(verifying_key.to_bytes());
        (signing_key, public_key_base64)
    }

    #[test]
    fn test_admin_error_status_codes() {
        use axum::response::IntoResponse;

        let crypto_err = AdminError::Crypto(CryptoError::InvalidPublicKey);
        let resp = crypto_err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let invalid_req = AdminError::InvalidRequest("bad".to_string());
        let resp = invalid_req.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let already = AdminError::AlreadyProvisioned;
        let resp = already.into_response();
        assert_eq!(resp.status(), StatusCode::CONFLICT);

        let deser = AdminError::DeserializationFailed("parse error".to_string());
        let resp = deser.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_admin_state_creation() {
        let (_, public_key_base64) = generate_admin_keys();
        let secret_store = create_secret_store();
        let state = AdminState::new(&public_key_base64, secret_store);
        assert!(state.is_ok());
    }

    #[test]
    fn test_admin_state_invalid_key() {
        let secret_store = create_secret_store();
        let result = AdminState::new("not-valid-base64!!!", secret_store);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_public_key() {
        let (_, public_key_base64) = generate_admin_keys();
        let secret_store = create_secret_store();
        let state = Arc::new(AdminState::new(&public_key_base64, secret_store).unwrap());

        let result = get_public_key(State(state)).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert!(!resp.0.public_key.is_empty());
    }

    #[tokio::test]
    async fn test_get_status_waiting() {
        let (_, public_key_base64) = generate_admin_keys();
        let secret_store = create_secret_store();
        let state = Arc::new(AdminState::new(&public_key_base64, secret_store).unwrap());

        let result = get_status(State(state)).await;
        assert_eq!(result.0.status, ProvisioningStatus::WaitingForProvisioning);
    }

    #[tokio::test]
    async fn test_provision_invalid_signature() {
        let (_, public_key_base64) = generate_admin_keys();
        let secret_store = create_secret_store();
        let state = Arc::new(AdminState::new(&public_key_base64, secret_store).unwrap());

        // Create a request with bogus signature
        let admin_x25519_secret = EphemeralSecret::random_from_rng(OsRng);
        let admin_x25519_public = x25519_dalek::PublicKey::from(&admin_x25519_secret);

        let request = ProvisionRequest {
            admin_x25519_public: BASE64.encode(admin_x25519_public.as_bytes()),
            ciphertext: BASE64.encode(b"fake ciphertext"),
            nonce: BASE64.encode([0u8; 12]),
            signature: BASE64.encode([0u8; 64]), // Invalid signature
        };

        let result = provision(State(state), Json(request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_provision_already_provisioned() {
        use crate::admin::crypto::encrypt_payload;
        use crate::admin::secrets::SecretsPayload;
        use ed25519_dalek::Signer;

        let (admin_signing_key, admin_public_key_base64) = generate_admin_keys();
        let secret_store = create_secret_store();
        let state = Arc::new(AdminState::new(&admin_public_key_base64, secret_store.clone()).unwrap());

        // First provision succeeds
        let bot_public_key_base64 = {
            let guard = state.keypair.read().await;
            guard.as_ref().unwrap().public_key_base64()
        };
        let bot_public_key = parse_x25519_public_key(&bot_public_key_base64).unwrap();

        let admin_x25519_secret = EphemeralSecret::random_from_rng(OsRng);
        let admin_x25519_public = x25519_dalek::PublicKey::from(&admin_x25519_secret);
        let shared_secret = admin_x25519_secret.diffie_hellman(&bot_public_key);

        let secrets = SecretsPayload {
            discord_token: "token".to_string(),
            hf_token: None,
            custom: Default::default(),
        };
        let plaintext = serde_json::to_vec(&secrets).unwrap();
        let (nonce, ciphertext) = encrypt_payload(&shared_secret, &plaintext).unwrap();

        let ciphertext_bytes = BASE64.decode(&ciphertext).unwrap();
        let nonce_bytes = BASE64.decode(&nonce).unwrap();
        let message = build_signature_message(
            admin_x25519_public.as_bytes(),
            &ciphertext_bytes,
            &nonce_bytes,
        );
        let signature = admin_signing_key.sign(&message);

        let request = ProvisionRequest {
            admin_x25519_public: BASE64.encode(admin_x25519_public.as_bytes()),
            ciphertext,
            nonce,
            signature: BASE64.encode(signature.to_bytes()),
        };
        provision(State(state.clone()), Json(request)).await.unwrap();

        // Second provision attempt should fail
        let admin_x25519_secret2 = EphemeralSecret::random_from_rng(OsRng);
        let admin_x25519_public2 = x25519_dalek::PublicKey::from(&admin_x25519_secret2);
        let request2 = ProvisionRequest {
            admin_x25519_public: BASE64.encode(admin_x25519_public2.as_bytes()),
            ciphertext: BASE64.encode(b"fake"),
            nonce: BASE64.encode([0u8; 12]),
            signature: BASE64.encode([0u8; 64]),
        };
        let result = provision(State(state), Json(request2)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_provision_flow() {
        use crate::admin::crypto::encrypt_payload;
        use ed25519_dalek::Signer;

        // Generate admin keys
        let (admin_signing_key, admin_public_key_base64) = generate_admin_keys();

        // Create admin state
        let secret_store = create_secret_store();
        let state = Arc::new(
            AdminState::new(&admin_public_key_base64, secret_store.clone()).unwrap(),
        );

        // Get bot's public key
        let bot_public_key_base64 = {
            let guard = state.keypair.read().await;
            guard.as_ref().unwrap().public_key_base64()
        };
        let bot_public_key = parse_x25519_public_key(&bot_public_key_base64).unwrap();

        // Admin generates ephemeral keypair for this session
        let admin_x25519_secret = EphemeralSecret::random_from_rng(OsRng);
        let admin_x25519_public = x25519_dalek::PublicKey::from(&admin_x25519_secret);

        // Compute shared secret
        let shared_secret = admin_x25519_secret.diffie_hellman(&bot_public_key);

        // Create secrets payload
        let secrets = SecretsPayload {
            discord_token: "test-discord-token".to_string(),
            hf_token: None,
            custom: Default::default(),
        };
        let plaintext = serde_json::to_vec(&secrets).unwrap();

        // Encrypt
        let (nonce, ciphertext) = encrypt_payload(&shared_secret, &plaintext).unwrap();

        // Build and sign message
        let ciphertext_bytes = BASE64.decode(&ciphertext).unwrap();
        let nonce_bytes = BASE64.decode(&nonce).unwrap();
        let message = build_signature_message(
            admin_x25519_public.as_bytes(),
            &ciphertext_bytes,
            &nonce_bytes,
        );
        let signature = admin_signing_key.sign(&message);

        // Create provision request
        let request = ProvisionRequest {
            admin_x25519_public: BASE64.encode(admin_x25519_public.as_bytes()),
            ciphertext,
            nonce,
            signature: BASE64.encode(signature.to_bytes()),
        };

        // Test status before provisioning
        assert_eq!(
            secret_store.status().await,
            ProvisioningStatus::WaitingForProvisioning
        );

        // Call provision handler
        let result = provision(State(state.clone()), Json(request)).await;
        assert!(result.is_ok());
        assert!(result.unwrap().0.success);

        // Verify provisioning succeeded
        assert_eq!(secret_store.status().await, ProvisioningStatus::Provisioned);
        assert_eq!(
            secret_store.discord_token().await,
            Some("test-discord-token".to_string())
        );
    }
}
