use std::fs;
use std::path::PathBuf;

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, AeadCore, Nonce,
};
use hkdf::Hkdf;
use sha2::Sha256;

use super::schema::AppConfig;

const HKDF_SALT: &[u8] = b"linguabridge-wallet-v1";
const HKDF_INFO: &[u8] = b"wallet-encryption-key";

pub struct ConfigStore {
    config_path: PathBuf,
    encrypted_path: PathBuf,
}

impl ConfigStore {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir()
            .ok_or("could not find config directory")?
            .join("linguabridge");

        fs::create_dir_all(&config_dir)?;

        Ok(Self {
            config_path: config_dir.join("config.json"),
            encrypted_path: config_dir.join("wallet.enc"),
        })
    }

    /// Load plaintext configuration from disk.
    pub fn load_config(&self) -> Result<AppConfig, Box<dyn std::error::Error>> {
        if !self.config_path.exists() {
            return Ok(AppConfig::default());
        }
        let data = fs::read_to_string(&self.config_path)?;
        let config: AppConfig = serde_json::from_str(&data)?;
        Ok(config)
    }

    /// Save plaintext configuration to disk.
    pub fn save_config(&self, config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::to_string_pretty(config)?;
        fs::write(&self.config_path, data)?;
        Ok(())
    }

    /// Load and decrypt the wallet mnemonic from disk using the given password.
    /// Panics on decryption failure (security: no recovery from crypto errors).
    pub fn load_wallet(&self, password: &str) -> Option<Vec<u8>> {
        if !self.encrypted_path.exists() {
            return None;
        }

        let data = fs::read(&self.encrypted_path)
            .expect("FATAL: cannot read wallet file");
        if data.len() < 12 {
            panic!("FATAL: wallet file corrupted (too short)");
        }

        let cipher = derive_cipher(password);
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .expect("FATAL: decryption failed — wrong password or corrupted wallet file");

        Some(plaintext)
    }

    /// Encrypt and save the wallet mnemonic to disk using the given password.
    /// Panics on encryption failure (security: no recovery from crypto errors).
    pub fn save_wallet(&self, mnemonic_bytes: &[u8], password: &str) {
        let cipher = derive_cipher(password);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let ciphertext = cipher
            .encrypt(&nonce, mnemonic_bytes)
            .expect("FATAL: encryption failed");

        let mut output = Vec::with_capacity(12 + ciphertext.len());
        output.extend_from_slice(&nonce);
        output.extend_from_slice(&ciphertext);

        fs::write(&self.encrypted_path, &output)
            .expect("FATAL: cannot write wallet file");
    }

    /// Check if an encrypted wallet file exists.
    pub fn has_wallet(&self) -> bool {
        self.encrypted_path.exists()
    }

    /// Path to the config directory.
    pub fn config_dir(&self) -> Option<&std::path::Path> {
        self.config_path.parent()
    }

    /// Path to the encrypted wallet file.
    pub fn wallet_path(&self) -> &std::path::Path {
        &self.encrypted_path
    }
}

/// Derive an AES-256-GCM cipher from a password using HKDF-SHA256.
/// Panics on failure (security: crypto primitives must not silently degrade).
fn derive_cipher(password: &str) -> Aes256Gcm {
    let hk = Hkdf::<Sha256>::new(Some(HKDF_SALT), password.as_bytes());
    let mut key = [0u8; 32];
    hk.expand(HKDF_INFO, &mut key)
        .expect("FATAL: HKDF expand failed");
    let cipher = Aes256Gcm::new_from_slice(&key)
        .expect("FATAL: failed to create AES cipher");
    key.fill(0);
    cipher
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_store() -> ConfigStore {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let tmp = env::temp_dir().join(format!("linguabridge-test-{}", id));
        fs::create_dir_all(&tmp).unwrap();
        ConfigStore {
            config_path: tmp.join("config.json"),
            encrypted_path: tmp.join("wallet.enc"),
        }
    }

    #[test]
    fn save_load_config_roundtrip() {
        let store = temp_store();
        let mut config = AppConfig::default();
        config.network.chain_id = "test-chain".to_string();
        store.save_config(&config).unwrap();

        let loaded = store.load_config().unwrap();
        assert_eq!(loaded.network.chain_id, "test-chain");

        // cleanup
        let _ = fs::remove_file(&store.config_path);
    }

    #[test]
    fn encrypt_decrypt_wallet_roundtrip() {
        let store = temp_store();
        let mnemonic = b"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
        let password = "test-password-123";

        store.save_wallet(mnemonic, password);
        assert!(store.has_wallet());

        let decrypted = store.load_wallet(password).unwrap();
        assert_eq!(decrypted, mnemonic);

        // cleanup
        let _ = fs::remove_file(&store.encrypted_path);
    }

    #[test]
    #[should_panic(expected = "FATAL: decryption failed")]
    fn wrong_password_panics() {
        let store = temp_store();
        store.save_wallet(b"secret mnemonic", "correct");

        // This should panic per security policy
        let _ = store.load_wallet("wrong");
    }

    #[test]
    fn load_missing_wallet_returns_none() {
        let store = temp_store();
        let _ = fs::remove_file(&store.encrypted_path);
        assert!(!store.has_wallet());
        assert!(store.load_wallet("any").is_none());
    }
}
