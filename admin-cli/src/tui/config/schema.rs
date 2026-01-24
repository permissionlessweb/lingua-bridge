// Placeholder for configuration schema
// Will be implemented in Phase 3

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub network: NetworkConfig,
    pub wallet: WalletConfig,
    pub deployments: Vec<SavedDeployment>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub chain_id: String,
    pub rpc_url: String,
    pub grpc_url: String,
    pub provider_url: String,
}

/// Default gRPC endpoint for Akash mainnet queries.
/// Using Polkachu's public endpoint (known working)
pub const DEFAULT_GRPC_URL: &str = "https://akash-grpc.polkachu.com:14490";

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            chain_id: "akashnet-2".to_string(),
            rpc_url: "https://rpc.akashnet.net:443".to_string(),
            grpc_url: DEFAULT_GRPC_URL.to_string(),
            provider_url: "https://provider.akashnet.net".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletConfig {
    pub encrypted_mnemonic: Option<Vec<u8>>,
    pub address: Option<String>,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            encrypted_mnemonic: None,
            address: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SavedDeployment {
    pub dseq: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            wallet: WalletConfig::default(),
            deployments: vec![],
        }
    }
}
