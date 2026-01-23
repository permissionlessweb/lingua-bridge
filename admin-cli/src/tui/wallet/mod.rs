pub mod keygen;
pub mod signer;

use zeroize::Zeroize;

/// Wallet holds the mnemonic and derived address.
pub struct Wallet {
    pub mnemonic: Option<String>,
    pub address: Option<String>,
}

impl Wallet {
    pub fn new() -> Self {
        Self {
            mnemonic: None,
            address: None,
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.mnemonic.is_some() && self.address.is_some()
    }
}

impl Drop for Wallet {
    fn drop(&mut self) {
        if let Some(ref mut m) = self.mnemonic {
            m.zeroize();
        }
    }
}

/// KeyPair holds raw key bytes for the secp256k1 key derived from the mnemonic.
/// `private_key` is the 32-byte scalar; `public_key` is the 33-byte compressed point.
pub struct KeyPair {
    pub public_key: Vec<u8>,
    pub private_key: Vec<u8>,
}

impl KeyPair {
    pub fn new() -> Self {
        Self {
            public_key: vec![],
            private_key: vec![],
        }
    }
}

impl Drop for KeyPair {
    fn drop(&mut self) {
        self.private_key.zeroize();
    }
}
