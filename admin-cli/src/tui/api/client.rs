use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use linguabridge_types::cosmos::bank::v1beta1::{
    query_client::QueryClient as BankQueryClient, QueryBalanceRequest,
};
use serde::{Deserialize, Serialize};

/// Deployment info from chain queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentInfo {
    pub owner: String,
    pub dseq: u64,
    pub state: String,
}

/// Bid info from market queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidInfo {
    pub provider: String,
    pub dseq: u64,
    pub gseq: u32,
    pub oseq: u32,
    pub price_amount: String,
    pub price_denom: String,
    pub state: String,
}

/// Lease info from market queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseInfo {
    pub owner: String,
    pub dseq: u64,
    pub gseq: u32,
    pub oseq: u32,
    pub provider: String,
    pub price_amount: String,
    pub price_denom: String,
    pub state: String,
}

/// Coin balance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub amount: String,
    pub denom: String,
}

/// Account info needed for tx signing.
#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub account_number: u64,
    pub sequence: u64,
}

/// Transaction broadcast result.
#[derive(Debug, Clone)]
pub struct BroadcastResult {
    pub txhash: String,
    pub code: u32,
    pub raw_log: String,
}

// --- Internal LCD response types ---

#[derive(Deserialize)]
struct LcdAccountResp {
    account: Option<LcdAccount>,
}

#[derive(Deserialize)]
struct LcdAccount {
    account_number: Option<String>,
    sequence: Option<String>,
    base_account: Option<Box<LcdAccount>>,
}

#[derive(Deserialize)]
struct LcdCoin {
    denom: String,
    amount: String,
}

#[derive(Deserialize)]
struct LcdDeploymentsResp {
    deployments: Option<Vec<LcdDeploymentEntry>>,
}

#[derive(Deserialize)]
struct LcdDeploymentEntry {
    deployment: LcdDeployment,
}

#[derive(Deserialize)]
struct LcdDeployment {
    deployment_id: LcdDeploymentId,
    state: String,
}

#[derive(Deserialize)]
struct LcdDeploymentId {
    owner: String,
    dseq: String,
}

#[derive(Deserialize)]
struct LcdBidsResp {
    bids: Option<Vec<LcdBidEntry>>,
}

#[derive(Deserialize)]
struct LcdBidEntry {
    bid: LcdBid,
}

#[derive(Deserialize)]
struct LcdBid {
    bid_id: LcdBidId,
    state: String,
    price: LcdDecCoin,
}

#[derive(Deserialize)]
struct LcdBidId {
    dseq: String,
    gseq: u32,
    oseq: u32,
    provider: String,
}

#[derive(Deserialize)]
struct LcdDecCoin {
    denom: String,
    amount: String,
}

#[derive(Deserialize)]
struct LcdLeasesResp {
    leases: Option<Vec<LcdLeaseEntry>>,
}

#[derive(Deserialize)]
struct LcdLeaseEntry {
    lease: LcdLease,
}

#[derive(Deserialize)]
struct LcdLease {
    lease_id: LcdLeaseId,
    state: String,
    price: LcdDecCoin,
}

#[derive(Deserialize)]
struct LcdLeaseId {
    owner: String,
    dseq: String,
    gseq: u32,
    oseq: u32,
    provider: String,
}

#[derive(Serialize)]
struct BroadcastTxReq {
    tx_bytes: String,
    mode: String,
}

#[derive(Deserialize)]
struct BroadcastTxResp {
    tx_response: Option<TxResp>,
}

#[derive(Deserialize)]
struct TxResp {
    txhash: Option<String>,
    code: Option<u32>,
    raw_log: Option<String>,
}

/// REST + gRPC client for Akash Network queries.
pub struct AkashClient {
    pub base_url: String,
    pub grpc_url: String,
    http: reqwest::Client,
}

impl AkashClient {
    pub fn new(base_url: String, grpc_url: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            grpc_url: grpc_url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Query account info (account_number, sequence) for tx signing.
    pub async fn get_account_info(
        &self,
        address: &str,
    ) -> Result<AccountInfo, Box<dyn std::error::Error>> {
        let url = format!("{}/cosmos/auth/v1beta1/accounts/{}", self.base_url, address);
        let resp: LcdAccountResp = self.http.get(&url).send().await?.json().await?;
        let account = resp.account.ok_or("account not found")?;
        let base = account.base_account.as_deref().unwrap_or(&account);
        Ok(AccountInfo {
            account_number: base
                .account_number
                .as_deref()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0),
            sequence: base.sequence.as_deref().unwrap_or("0").parse().unwrap_or(0),
        })
    }

    /// Query balance for an address via gRPC. Returns the uakt balance.
    pub async fn query_balance(
        &self,
        address: &str,
    ) -> Result<Balance, Box<dyn std::error::Error>> {
        let channel = tonic::transport::Channel::from_shared(self.grpc_url.clone())?
            .connect()
            .await?;
        let mut client = BankQueryClient::new(channel);
        let resp = client
            .balance(QueryBalanceRequest {
                address: address.to_string(),
                denom: "uakt".to_string(),
            })
            .await?;
        let coin = resp.into_inner().balance.unwrap_or_default();
        Ok(Balance {
            denom: if coin.denom.is_empty() {
                "uakt".to_string()
            } else {
                coin.denom
            },
            amount: if coin.amount.is_empty() {
                "0".to_string()
            } else {
                coin.amount
            },
        })
    }

    /// Query deployments owned by the given address.
    pub async fn query_deployments(
        &self,
        owner: &str,
    ) -> Result<Vec<DeploymentInfo>, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/akash/deployment/v1beta3/deployments/list?filters.owner={}",
            self.base_url, owner
        );
        let resp: LcdDeploymentsResp = self.http.get(&url).send().await?.json().await?;
        Ok(resp
            .deployments
            .unwrap_or_default()
            .into_iter()
            .map(|e| DeploymentInfo {
                owner: e.deployment.deployment_id.owner,
                dseq: e.deployment.deployment_id.dseq.parse().unwrap_or(0),
                state: e.deployment.state,
            })
            .collect())
    }

    /// Query bids for a specific deployment.
    pub async fn query_bids(
        &self,
        owner: &str,
        dseq: u64,
    ) -> Result<Vec<BidInfo>, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/akash/market/v1beta4/bids/list?filters.owner={}&id.dseq={}",
            self.base_url, owner, dseq
        );
        let resp: LcdBidsResp = self.http.get(&url).send().await?.json().await?;
        Ok(resp
            .bids
            .unwrap_or_default()
            .into_iter()
            .map(|e| BidInfo {
                provider: e.bid.bid_id.provider,
                dseq: e.bid.bid_id.dseq.parse().unwrap_or(0),
                gseq: e.bid.bid_id.gseq,
                oseq: e.bid.bid_id.oseq,
                price_amount: e.bid.price.amount,
                price_denom: e.bid.price.denom,
                state: e.bid.state,
            })
            .collect())
    }

    /// Query active leases for an address.
    pub async fn query_leases(
        &self,
        owner: &str,
    ) -> Result<Vec<LeaseInfo>, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/akash/market/v1beta4/leases/list?filters.owner={}",
            self.base_url, owner
        );
        let resp: LcdLeasesResp = self.http.get(&url).send().await?.json().await?;
        Ok(resp
            .leases
            .unwrap_or_default()
            .into_iter()
            .map(|e| LeaseInfo {
                owner: e.lease.lease_id.owner,
                dseq: e.lease.lease_id.dseq.parse().unwrap_or(0),
                gseq: e.lease.lease_id.gseq,
                oseq: e.lease.lease_id.oseq,
                provider: e.lease.lease_id.provider,
                price_amount: e.lease.price.amount,
                price_denom: e.lease.price.denom,
                state: e.lease.state,
            })
            .collect())
    }

    /// Broadcast a signed transaction (BROADCAST_MODE_SYNC).
    pub async fn broadcast_tx(
        &self,
        tx_bytes: &[u8],
    ) -> Result<BroadcastResult, Box<dyn std::error::Error>> {
        let url = format!("{}/cosmos/tx/v1beta1/txs", self.base_url);
        let req = BroadcastTxReq {
            tx_bytes: BASE64.encode(tx_bytes),
            mode: "BROADCAST_MODE_SYNC".to_string(),
        };
        let resp: BroadcastTxResp = self.http.post(&url).json(&req).send().await?.json().await?;
        let tx_resp = resp
            .tx_response
            .ok_or("no tx_response in broadcast result")?;
        Ok(BroadcastResult {
            txhash: tx_resp.txhash.unwrap_or_default(),
            code: tx_resp.code.unwrap_or(0),
            raw_log: tx_resp.raw_log.unwrap_or_default(),
        })
    }

    /// Get the latest block height (useful for dseq).
    pub async fn get_block_height(&self) -> Result<u64, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/cosmos/base/tendermint/v1beta1/blocks/latest",
            self.base_url
        );
        let resp: serde_json::Value = self.http.get(&url).send().await?.json().await?;
        let height = resp["block"]["header"]["height"]
            .as_str()
            .unwrap_or("0")
            .parse::<u64>()?;
        Ok(height)
    }

    /// Wait for a tx to be included in a block (polls every 2s).
    pub async fn wait_for_tx(
        &self,
        txhash: &str,
        timeout_secs: u64,
    ) -> Result<BroadcastResult, Box<dyn std::error::Error>> {
        let url = format!("{}/cosmos/tx/v1beta1/txs/{}", self.base_url, txhash);
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(timeout_secs);

        loop {
            if tokio::time::Instant::now() > deadline {
                return Err(format!("timeout waiting for tx {}", txhash).into());
            }
            if let Ok(response) = self.http.get(&url).send().await {
                if response.status().is_success() {
                    if let Ok(body) = response.json::<serde_json::Value>().await {
                        if let Some(tx_resp) = body.get("tx_response") {
                            return Ok(BroadcastResult {
                                txhash: tx_resp["txhash"].as_str().unwrap_or(txhash).to_string(),
                                code: tx_resp["code"].as_u64().unwrap_or(0) as u32,
                                raw_log: tx_resp["raw_log"].as_str().unwrap_or("").to_string(),
                            });
                        }
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }
}

/// Fee grant allowance info.
#[derive(Debug, Clone)]
pub struct FeeAllowanceInfo {
    pub granter: String,
    pub spend_limit: Option<Balance>,
    pub expiration: Option<String>,
}

#[derive(Deserialize)]
struct LcdAllowancesResp {
    allowances: Option<Vec<LcdAllowanceEntry>>,
}

#[derive(Deserialize)]
struct LcdAllowanceEntry {
    granter: String,
    allowance: Option<LcdAllowanceValue>,
}

#[derive(Deserialize)]
struct LcdAllowanceValue {
    spend_limit: Option<Vec<LcdCoin>>,
    expiration: Option<String>,
    // For AllowedMsgAllowance, the inner allowance is nested
    allowance: Option<Box<LcdAllowanceValue>>,
}

impl AkashClient {
    /// Query fee grant allowances for a grantee address.
    pub async fn query_fee_allowances(
        &self,
        grantee: &str,
    ) -> Result<Vec<FeeAllowanceInfo>, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/cosmos/feegrant/v1beta1/allowances/{}",
            self.base_url, grantee
        );
        let resp: LcdAllowancesResp = self.http.get(&url).send().await?.json().await?;
        Ok(resp
            .allowances
            .unwrap_or_default()
            .into_iter()
            .map(|entry| {
                // Handle nested allowance (AllowedMsgAllowance wraps BasicAllowance)
                let inner = entry
                    .allowance
                    .as_ref()
                    .and_then(|a| a.allowance.as_deref())
                    .or(entry.allowance.as_ref());

                let spend_limit = inner
                    .and_then(|a| a.spend_limit.as_ref())
                    .and_then(|coins| coins.iter().find(|c| c.denom == "uakt"))
                    .map(|c| Balance {
                        amount: c.amount.clone(),
                        denom: c.denom.clone(),
                    });

                let expiration = inner.and_then(|a| a.expiration.clone());

                FeeAllowanceInfo {
                    granter: entry.granter,
                    spend_limit,
                    expiration,
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_trims_trailing_slash() {
        let client = AkashClient::new(
            "https://api.akashnet.net/".to_string(),
            "https://grpc.akashnet.net:443/".to_string(),
        );
        assert_eq!(client.base_url, "https://api.akashnet.net");
        assert_eq!(client.grpc_url, "https://grpc.akashnet.net:443");
    }
}
