use super::{KeyPair, Wallet};
use bip39::Mnemonic;
use cosmrs::bip32::DerivationPath;
use cosmrs::crypto::secp256k1::SigningKey;
use rand::RngCore;

/// Cosmos SDK HD derivation path for Akash (coin type 118).
const AKASH_HD_PATH: &str = "m/44'/118'/0'/0/0";
const AKASH_BECH32_PREFIX: &str = "akash";

pub struct KeyGenerator;

impl KeyGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generate a new 24-word BIP-39 mnemonic using OS entropy.
    pub fn generate_mnemonic(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut entropy = [0u8; 32]; // 256 bits = 24 words
        rand::thread_rng().fill_bytes(&mut entropy);
        let mnemonic = Mnemonic::from_entropy(&entropy)
            .map_err(|e| format!("mnemonic generation failed: {}", e))?;
        Ok(mnemonic.to_string())
    }

    /// Derive a secp256k1 keypair from a BIP-39 mnemonic using the Akash HD path.
    /// Returns raw key bytes: private_key (32 bytes), public_key (33 bytes compressed).
    pub fn derive_keypair(&self, mnemonic: &str) -> Result<KeyPair, Box<dyn std::error::Error>> {
        let parsed: Mnemonic = mnemonic
            .parse()
            .map_err(|e| format!("invalid mnemonic: {}", e))?;

        let seed = parsed.to_seed("");
        let path: DerivationPath = AKASH_HD_PATH
            .parse()
            .map_err(|e| format!("invalid HD path: {}", e))?;

        // Derive using bip32 XPrv to access raw key bytes
        let child_xprv = cosmrs::bip32::XPrv::derive_from_path(seed, &path)
            .map_err(|e| format!("key derivation failed: {}", e))?;

        // Extract the 32-byte private key from the extended private key
        let private_key_bytes: Vec<u8> = child_xprv.private_key().to_bytes().to_vec();

        // Construct cosmrs SigningKey to derive the compressed public key
        let signing_key = SigningKey::from_slice(&private_key_bytes)
            .map_err(|e| format!("failed to create signing key: {}", e))?;
        let public_key = signing_key.public_key();

        Ok(KeyPair {
            private_key: private_key_bytes,
            public_key: public_key.to_bytes().to_vec(),
        })
    }

    /// Derive the bech32 akash1... address from a keypair.
    /// Uses the private key to reconstruct the signing key and derive the account ID.
    pub fn derive_address(&self, keypair: &KeyPair) -> Result<String, Box<dyn std::error::Error>> {
        let signing_key = SigningKey::from_slice(&keypair.private_key)
            .map_err(|e| format!("invalid private key: {}", e))?;
        let account_id = signing_key
            .public_key()
            .account_id(AKASH_BECH32_PREFIX)
            .map_err(|e| format!("failed to derive address: {}", e))?;
        Ok(account_id.to_string())
    }

    /// Create a wallet from a mnemonic: generates keypair + derives address.
    pub fn create_wallet(&self, mnemonic: String) -> Result<Wallet, Box<dyn std::error::Error>> {
        let keypair = self.derive_keypair(&mnemonic)?;
        let address = self.derive_address(&keypair)?;

        Ok(Wallet {
            mnemonic: Some(mnemonic),
            address: Some(address),
        })
    }

    /// Import and validate an existing mnemonic, then create the wallet.
    pub fn import_wallet(&self, mnemonic: String) -> Result<Wallet, Box<dyn std::error::Error>> {
        self.validate_mnemonic(&mnemonic)?;
        self.create_wallet(mnemonic)
    }

    /// Validate that a string is a valid BIP-39 mnemonic.
    pub fn validate_mnemonic(&self, mnemonic: &str) -> Result<(), Box<dyn std::error::Error>> {
        let _: Mnemonic = mnemonic
            .parse()
            .map_err(|e| format!("invalid mnemonic: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_mnemonic_produces_24_words() {
        let gen = KeyGenerator::new();
        let mnemonic = gen.generate_mnemonic().unwrap();
        assert_eq!(mnemonic.split_whitespace().count(), 24);
    }

    #[test]
    fn derive_keypair_produces_correct_lengths() {
        let gen = KeyGenerator::new();
        let mnemonic = gen.generate_mnemonic().unwrap();
        let keypair = gen.derive_keypair(&mnemonic).unwrap();
        assert_eq!(keypair.private_key.len(), 32);
        assert_eq!(keypair.public_key.len(), 33);
    }

    #[test]
    fn derive_address_produces_akash_prefix() {
        let gen = KeyGenerator::new();
        let mnemonic = gen.generate_mnemonic().unwrap();
        let keypair = gen.derive_keypair(&mnemonic).unwrap();
        let address = gen.derive_address(&keypair).unwrap();
        assert!(address.starts_with("akash1"));
    }

    #[test]
    fn create_wallet_end_to_end() {
        let gen = KeyGenerator::new();
        let mnemonic = gen.generate_mnemonic().unwrap();
        let wallet = gen.create_wallet(mnemonic.clone()).unwrap();
        assert!(wallet.is_loaded());
        assert!(wallet.address.as_ref().unwrap().starts_with("akash1"));
        assert_eq!(wallet.mnemonic.as_ref().unwrap(), &mnemonic);
    }

    #[test]
    fn deterministic_derivation() {
        let gen = KeyGenerator::new();
        let mnemonic = gen.generate_mnemonic().unwrap();
        let wallet1 = gen.create_wallet(mnemonic.clone()).unwrap();
        let wallet2 = gen.create_wallet(mnemonic).unwrap();
        assert_eq!(wallet1.address, wallet2.address);
    }

    #[test]
    fn validate_mnemonic_rejects_garbage() {
        let gen = KeyGenerator::new();
        assert!(gen.validate_mnemonic("not a valid mnemonic phrase").is_err());
    }

    #[test]
    fn import_wallet_validates_first() {
        let gen = KeyGenerator::new();
        assert!(gen.import_wallet("bad phrase".to_string()).is_err());
    }
}
