//! Secure admin transport layer for provisioning secrets.
//!
//! This module provides cryptographically secured configuration delivery,
//! allowing administrators to provision sensitive secrets (like Discord tokens)
//! to running instances without exposing them via environment variables.
//!
//! ## Security Model
//!
//! - Bot generates an ephemeral X25519 keypair on each startup
//! - Admin signs encrypted payloads with their Ed25519 key
//! - Secrets are stored only in memory (never persisted)
//! - Memory is zeroized on drop
//!
//! ## Usage
//!
//! ```ignore
//! // Create secret store
//! let secret_store = admin::secrets::create_secret_store();
//!
//! // Create admin state with admin's public key
//! let admin_state = AdminState::new(
//!     &config.admin.public_key,
//!     secret_store.clone(),
//! )?;
//!
//! // Start admin server and wait for provisioning
//! let admin_router = admin::transport::admin_router(Arc::new(admin_state));
//! // ... start server ...
//!
//! // Wait for secrets
//! secret_store.wait_for_provisioning().await;
//!
//! // Use secrets
//! let discord_token = secret_store.discord_token().await.unwrap();
//! ```

pub mod crypto;
pub mod secrets;
pub mod transport;

pub use crypto::{CryptoError, EphemeralKeyPair};
pub use secrets::{create_secret_store, ProvisioningStatus, SecretsPayload, SecretStore, SharedSecretStore};
pub use transport::{admin_router, AdminState};
