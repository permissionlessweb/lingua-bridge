use serde::{Deserialize, Serialize};

/// Service status from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub available: i32,
    pub total: i32,
    pub uris: Vec<String>,
    pub ready_replicas: i32,
}

/// Forwarded port from a provider lease.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardedPort {
    pub host: String,
    pub port: u32,
    pub external_port: u32,
    pub proto: String,
}

/// Log entry from a service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub name: String,
    pub message: String,
}

/// Client for interacting with Akash provider REST APIs.
pub struct ProviderClient {
    http: reqwest::Client,
}

impl ProviderClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    /// Send the deployment manifest to the provider after lease creation.
    pub async fn send_manifest(
        &self,
        provider_url: &str,
        dseq: u64,
        manifest_json: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "{}/deployment/{}/manifest",
            provider_url.trim_end_matches('/'),
            dseq
        );
        let resp = self.http.put(&url).json(manifest_json).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("provider rejected manifest ({}): {}", status, body).into());
        }
        Ok(())
    }

    /// Query lease status from the provider.
    pub async fn get_status(
        &self,
        provider_url: &str,
        dseq: u64,
        gseq: u32,
        oseq: u32,
    ) -> Result<Vec<ServiceStatus>, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/lease/{}/{}/{}/status",
            provider_url.trim_end_matches('/'),
            dseq, gseq, oseq
        );
        let resp: serde_json::Value = self.http.get(&url).send().await?.json().await?;
        let mut services = Vec::new();

        if let Some(svcs) = resp.get("services") {
            if let Some(obj) = svcs.as_object() {
                for (name, info) in obj {
                    services.push(ServiceStatus {
                        name: name.clone(),
                        available: info["available"].as_i64().unwrap_or(0) as i32,
                        total: info["total"].as_i64().unwrap_or(0) as i32,
                        uris: info["uris"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        ready_replicas: info["ready_replicas"].as_i64().unwrap_or(0) as i32,
                    });
                }
            }
        }

        Ok(services)
    }

    /// Get forwarded ports for a lease.
    pub async fn get_forwarded_ports(
        &self,
        provider_url: &str,
        dseq: u64,
        gseq: u32,
        oseq: u32,
    ) -> Result<Vec<ForwardedPort>, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/lease/{}/{}/{}/status",
            provider_url.trim_end_matches('/'),
            dseq, gseq, oseq
        );
        let resp: serde_json::Value = self.http.get(&url).send().await?.json().await?;
        let mut ports = Vec::new();

        if let Some(fwd) = resp.get("forwarded_ports") {
            if let Some(obj) = fwd.as_object() {
                for (_service, port_list) in obj {
                    if let Some(arr) = port_list.as_array() {
                        for p in arr {
                            ports.push(ForwardedPort {
                                host: p["host"].as_str().unwrap_or("").to_string(),
                                port: p["port"].as_u64().unwrap_or(0) as u32,
                                external_port: p["externalPort"].as_u64().unwrap_or(0) as u32,
                                proto: p["proto"].as_str().unwrap_or("TCP").to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(ports)
    }

    /// Get service logs from the provider.
    pub async fn get_logs(
        &self,
        provider_url: &str,
        dseq: u64,
        gseq: u32,
        oseq: u32,
        service_name: &str,
        tail: u64,
    ) -> Result<Vec<LogEntry>, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/lease/{}/{}/{}/logs?service={}&tail={}",
            provider_url.trim_end_matches('/'),
            dseq, gseq, oseq, service_name, tail
        );
        let resp = self.http.get(&url).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("failed to get logs ({}): {}", status, body).into());
        }

        let body = resp.text().await?;
        let entries: Vec<LogEntry> = body
            .lines()
            .map(|line| LogEntry {
                name: service_name.to_string(),
                message: line.to_string(),
            })
            .collect();

        Ok(entries)
    }
}

impl Default for ProviderClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_client_creation() {
        let _client = ProviderClient::new();
    }
}
