use super::KeyPair;
use cosmrs::crypto::secp256k1::SigningKey;
use cosmrs::tx::{Body, Fee, SignDoc, SignerInfo};
use cosmrs::{Any, Coin};

const AKASH_BECH32_PREFIX: &str = "akash";

pub struct TransactionSigner {
    keypair: KeyPair,
}

impl TransactionSigner {
    pub fn new(keypair: KeyPair) -> Self {
        Self { keypair }
    }

    /// Reconstruct the cosmrs SigningKey from raw bytes.
    fn signing_key(&self) -> Result<SigningKey, Box<dyn std::error::Error>> {
        let key = SigningKey::from_slice(&self.keypair.private_key)
            .map_err(|e| format!("failed to load signing key: {}", e))?;
        Ok(key)
    }

    /// Get the bech32 akash address for this signer.
    pub fn address(&self) -> Result<String, Box<dyn std::error::Error>> {
        let sk = self.signing_key()?;
        let account_id = sk
            .public_key()
            .account_id(AKASH_BECH32_PREFIX)
            .map_err(|e| format!("failed to derive address: {}", e))?;
        Ok(account_id.to_string())
    }

    /// Sign raw bytes with the secp256k1 key (low-level).
    pub fn sign_transaction(&self, tx_doc: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let sk = self.signing_key()?;
        let signature = sk
            .sign(tx_doc)
            .map_err(|e| format!("signing failed: {}", e))?;
        Ok(signature.to_bytes().to_vec())
    }

    /// Build and sign a Cosmos SDK transaction, returning the encoded tx bytes
    /// ready for broadcast.
    ///
    /// `messages` - protobuf-encoded Any messages (use `encode_msg` helper)
    /// `chain_id` - e.g. "akashnet-2"
    /// `account_number` - from /cosmos/auth/v1beta1/accounts/{addr}
    /// `sequence` - from account query
    /// `gas_limit` - gas units
    /// `fee_amount` - fee in uakt
    /// `memo` - optional memo
    pub fn create_signed_tx(
        &self,
        messages: Vec<Any>,
        chain_id: &str,
        account_number: u64,
        sequence: u64,
        gas_limit: u64,
        fee_amount: u128,
        memo: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let sk = self.signing_key()?;

        let fee_coin = Coin {
            denom: "uakt"
                .parse()
                .map_err(|_| "invalid fee denom")?,
            amount: fee_amount,
        };

        let tx_body = Body::new(messages, memo, 0u32);

        let auth_info = SignerInfo::single_direct(Some(sk.public_key()), sequence)
            .auth_info(Fee::from_amount_and_gas(fee_coin, gas_limit));

        let chain_id_parsed = chain_id
            .parse()
            .map_err(|_| format!("invalid chain ID: {}", chain_id))?;

        let sign_doc = SignDoc::new(&tx_body, &auth_info, &chain_id_parsed, account_number)
            .map_err(|e| format!("failed to build sign doc: {}", e))?;

        let tx_signed = sign_doc
            .sign(&sk)
            .map_err(|e| format!("signing failed: {}", e))?;

        let tx_bytes = tx_signed
            .to_bytes()
            .map_err(|e| format!("failed to encode tx: {}", e))?;

        Ok(tx_bytes)
    }

    /// Encode a prost Message into a Cosmos SDK `Any` for use in transactions.
    pub fn encode_msg<M: prost::Message + prost::Name>(msg: &M) -> Result<Any, Box<dyn std::error::Error>> {
        let type_url = M::type_url();
        let mut buf = Vec::new();
        msg.encode(&mut buf)
            .map_err(|e| format!("protobuf encode failed: {}", e))?;
        Ok(Any {
            type_url,
            value: buf,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::wallet::keygen::KeyGenerator;

    #[test]
    fn signer_from_generated_keypair() {
        let gen = KeyGenerator::new();
        let mnemonic = gen.generate_mnemonic().unwrap();
        let keypair = gen.derive_keypair(&mnemonic).unwrap();
        let signer = TransactionSigner::new(keypair);
        let address = signer.address().unwrap();
        assert!(address.starts_with("akash1"));
    }

    #[test]
    fn sign_empty_tx() {
        let gen = KeyGenerator::new();
        let mnemonic = gen.generate_mnemonic().unwrap();
        let keypair = gen.derive_keypair(&mnemonic).unwrap();
        let signer = TransactionSigner::new(keypair);

        let tx_bytes = signer
            .create_signed_tx(vec![], "akashnet-2", 0, 0, 200_000, 5000, "")
            .unwrap();
        assert!(!tx_bytes.is_empty());
    }

    #[test]
    fn sign_raw_bytes() {
        let gen = KeyGenerator::new();
        let mnemonic = gen.generate_mnemonic().unwrap();
        let keypair = gen.derive_keypair(&mnemonic).unwrap();
        let signer = TransactionSigner::new(keypair);

        let sig = signer.sign_transaction(b"test message").unwrap();
        assert!(!sig.is_empty());
    }
}
