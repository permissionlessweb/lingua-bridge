//! Cryptographic primitives for secure admin transport.
//!
//! Uses:
//! - Ed25519 for admin signature verification
//! - X25519 for ephemeral key exchange
//! - ChaCha20-Poly1305 for authenticated encryption

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use thiserror::Error;
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret};

/// Cryptographic errors
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Invalid base64 encoding: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("Invalid public key format")]
    InvalidPublicKey,

    #[error("Invalid signature format")]
    InvalidSignature,

    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Decryption failed - invalid ciphertext or wrong key")]
    DecryptionFailed,

    #[error("Invalid nonce length")]
    InvalidNonce,
}

/// Result type for crypto operations
pub type CryptoResult<T> = Result<T, CryptoError>;

/// Ephemeral X25519 keypair for session encryption.
///
/// The private key is held only in memory and regenerated on each boot.
pub struct EphemeralKeyPair {
    secret: EphemeralSecret,
    public: PublicKey,
}

impl EphemeralKeyPair {
    /// Generate a new ephemeral keypair.
    pub fn generate() -> Self {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Get the public key as bytes.
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.public.to_bytes()
    }

    /// Get the public key as base64.
    pub fn public_key_base64(&self) -> String {
        BASE64.encode(self.public.to_bytes())
    }

    /// Perform Diffie-Hellman key exchange with the admin's public key.
    pub fn diffie_hellman(self, their_public: &PublicKey) -> SharedSecret {
        self.secret.diffie_hellman(their_public)
    }
}

/// Parse an X25519 public key from base64.
pub fn parse_x25519_public_key(base64_key: &str) -> CryptoResult<PublicKey> {
    let bytes = BASE64.decode(base64_key)?;
    if bytes.len() != 32 {
        return Err(CryptoError::InvalidPublicKey);
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(PublicKey::from(arr))
}

/// Parse an Ed25519 verifying key from base64.
pub fn parse_ed25519_public_key(base64_key: &str) -> CryptoResult<VerifyingKey> {
    let bytes = BASE64.decode(base64_key)?;
    if bytes.len() != 32 {
        return Err(CryptoError::InvalidPublicKey);
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    VerifyingKey::from_bytes(&arr).map_err(|_| CryptoError::InvalidPublicKey)
}

/// Parse an Ed25519 signature from base64.
pub fn parse_signature(base64_sig: &str) -> CryptoResult<Signature> {
    let bytes = BASE64.decode(base64_sig)?;
    if bytes.len() != 64 {
        return Err(CryptoError::InvalidSignature);
    }
    let mut arr = [0u8; 64];
    arr.copy_from_slice(&bytes);
    Ok(Signature::from_bytes(&arr))
}

/// Verify an Ed25519 signature.
///
/// The message should be: admin_x25519_public || ciphertext || nonce
pub fn verify_signature(
    admin_public_key: &VerifyingKey,
    message: &[u8],
    signature: &Signature,
) -> CryptoResult<()> {
    admin_public_key
        .verify(message, signature)
        .map_err(|_| CryptoError::SignatureVerificationFailed)
}

/// Decrypt a ChaCha20-Poly1305 ciphertext using a shared secret.
pub fn decrypt_payload(
    shared_secret: &SharedSecret,
    nonce_base64: &str,
    ciphertext_base64: &str,
) -> CryptoResult<Vec<u8>> {
    let nonce_bytes = BASE64.decode(nonce_base64)?;
    if nonce_bytes.len() != 12 {
        return Err(CryptoError::InvalidNonce);
    }

    let ciphertext = BASE64.decode(ciphertext_base64)?;

    // Derive encryption key from shared secret (use as-is since it's already 32 bytes)
    let cipher = ChaCha20Poly1305::new_from_slice(shared_secret.as_bytes())
        .map_err(|_| CryptoError::DecryptionFailed)?;

    let nonce = Nonce::from_slice(&nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| CryptoError::DecryptionFailed)
}

/// Encrypt a payload using ChaCha20-Poly1305 (for admin CLI).
pub fn encrypt_payload(shared_secret: &SharedSecret, plaintext: &[u8]) -> CryptoResult<(String, String)> {
    use rand::RngCore;

    // Generate random nonce
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Create cipher
    let cipher = ChaCha20Poly1305::new_from_slice(shared_secret.as_bytes())
        .map_err(|_| CryptoError::DecryptionFailed)?;

    // Encrypt
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| CryptoError::DecryptionFailed)?;

    Ok((BASE64.encode(nonce_bytes), BASE64.encode(ciphertext)))
}

/// Build the message to sign: admin_x25519_public || ciphertext || nonce
pub fn build_signature_message(
    admin_x25519_public: &[u8],
    ciphertext: &[u8],
    nonce: &[u8],
) -> Vec<u8> {
    let mut message = Vec::with_capacity(admin_x25519_public.len() + ciphertext.len() + nonce.len());
    message.extend_from_slice(admin_x25519_public);
    message.extend_from_slice(ciphertext);
    message.extend_from_slice(nonce);
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ephemeral_keypair_generation() {
        let kp1 = EphemeralKeyPair::generate();
        let kp2 = EphemeralKeyPair::generate();

        // Public keys should be different
        assert_ne!(kp1.public_key_bytes(), kp2.public_key_bytes());

        // Base64 encoding should work
        let b64 = kp1.public_key_base64();
        assert!(!b64.is_empty());

        // Should be able to parse it back
        let parsed = parse_x25519_public_key(&b64).unwrap();
        assert_eq!(parsed.to_bytes(), kp1.public_key_bytes());
    }

    #[test]
    fn test_key_exchange_and_encryption() {
        // Simulate bot and admin keypairs
        let bot_kp = EphemeralKeyPair::generate();
        let admin_secret = EphemeralSecret::random_from_rng(OsRng);
        let admin_public = PublicKey::from(&admin_secret);

        // Admin computes shared secret
        let admin_shared = admin_secret.diffie_hellman(&bot_kp.public);

        // Bot computes shared secret
        let bot_shared = bot_kp.diffie_hellman(&admin_public);

        // Shared secrets should match
        assert_eq!(admin_shared.as_bytes(), bot_shared.as_bytes());

        // Test encryption/decryption
        let plaintext = b"Hello, World!";
        let (nonce, ciphertext) = encrypt_payload(&admin_shared, plaintext).unwrap();

        let decrypted = decrypt_payload(&bot_shared, &nonce, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
