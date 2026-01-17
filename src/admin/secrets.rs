//! In-memory secret storage with secure memory handling.
//!
//! Secrets are stored only in RAM and are zeroized on drop to minimize
//! exposure window. Never persisted to disk.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};
use zeroize::Zeroize;

/// Secrets payload sent by admin during provisioning.
/// This structure is serialized/deserialized for transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsPayload {
    /// Discord bot token
    pub discord_token: String,
    /// Optional: Hugging Face API token for inference
    #[serde(default)]
    pub hf_token: Option<String>,
    /// Optional: Additional custom secrets as key-value pairs
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

impl Drop for SecretsPayload {
    fn drop(&mut self) {
        // Zeroize the string fields
        self.discord_token.zeroize();
        if let Some(ref mut token) = self.hf_token {
            token.zeroize();
        }
        // Zeroize HashMap values (keys are not sensitive)
        for value in self.custom.values_mut() {
            value.zeroize();
        }
        self.custom.clear();
    }
}

/// Current provisioning status of the bot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvisioningStatus {
    /// Waiting for admin to provision secrets
    WaitingForProvisioning,
    /// Secrets received, application running
    Provisioned,
}

/// In-memory secret store.
///
/// This store holds sensitive configuration that was securely transmitted
/// by the admin. Secrets are zeroized when the store is dropped.
pub struct SecretStore {
    /// The actual secrets (None until provisioned)
    secrets: RwLock<Option<SecretsPayload>>,
    /// Notification channel for when secrets arrive
    provisioned_notify: Notify,
}

impl SecretStore {
    /// Create a new empty secret store.
    pub fn new() -> Self {
        Self {
            secrets: RwLock::new(None),
            provisioned_notify: Notify::new(),
        }
    }

    /// Check if the store has been provisioned with secrets.
    pub async fn is_provisioned(&self) -> bool {
        self.secrets.read().await.is_some()
    }

    /// Get the current provisioning status.
    pub async fn status(&self) -> ProvisioningStatus {
        if self.is_provisioned().await {
            ProvisioningStatus::Provisioned
        } else {
            ProvisioningStatus::WaitingForProvisioning
        }
    }

    /// Store secrets (called after successful decryption and verification).
    ///
    /// Returns false if already provisioned (can only provision once).
    pub async fn provision(&self, secrets: SecretsPayload) -> bool {
        let mut guard = self.secrets.write().await;
        if guard.is_some() {
            // Already provisioned, reject
            return false;
        }
        *guard = Some(secrets);
        drop(guard);

        // Notify waiters that secrets are available
        self.provisioned_notify.notify_waiters();
        true
    }

    /// Wait until secrets are provisioned.
    ///
    /// This is used by the main application to block startup until
    /// the admin has provided credentials.
    pub async fn wait_for_provisioning(&self) {
        // Check if already provisioned
        if self.is_provisioned().await {
            return;
        }

        // Wait for notification
        self.provisioned_notify.notified().await;
    }

    /// Get the Discord token.
    ///
    /// Returns None if not yet provisioned.
    pub async fn discord_token(&self) -> Option<String> {
        self.secrets
            .read()
            .await
            .as_ref()
            .map(|s| s.discord_token.clone())
    }

    /// Get the Hugging Face token.
    pub async fn hf_token(&self) -> Option<String> {
        self.secrets
            .read()
            .await
            .as_ref()
            .and_then(|s| s.hf_token.clone())
    }

    /// Get a custom secret by key.
    pub async fn custom_secret(&self, key: &str) -> Option<String> {
        self.secrets
            .read()
            .await
            .as_ref()
            .and_then(|s| s.custom.get(key).cloned())
    }
}

impl Default for SecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SecretStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretStore")
            .field("provisioned", &"<check async>")
            .finish_non_exhaustive()
    }
}

/// Shared secret store handle for use across the application.
pub type SharedSecretStore = Arc<SecretStore>;

/// Create a new shared secret store.
pub fn create_secret_store() -> SharedSecretStore {
    Arc::new(SecretStore::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_secret_store_lifecycle() {
        let store = SecretStore::new();

        // Initially not provisioned
        assert!(!store.is_provisioned().await);
        assert_eq!(store.status().await, ProvisioningStatus::WaitingForProvisioning);
        assert!(store.discord_token().await.is_none());

        // Provision
        let secrets = SecretsPayload {
            discord_token: "test-token".to_string(),
            hf_token: None,
            custom: Default::default(),
        };
        assert!(store.provision(secrets).await);

        // Now provisioned
        assert!(store.is_provisioned().await);
        assert_eq!(store.status().await, ProvisioningStatus::Provisioned);
        assert_eq!(store.discord_token().await, Some("test-token".to_string()));

        // Cannot provision again
        let secrets2 = SecretsPayload {
            discord_token: "another-token".to_string(),
            hf_token: None,
            custom: Default::default(),
        };
        assert!(!store.provision(secrets2).await);

        // Token unchanged
        assert_eq!(store.discord_token().await, Some("test-token".to_string()));
    }
}
