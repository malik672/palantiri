use async_trait::async_trait;
use bytes::Bytes;
use reqwest::{Client, ClientBuilder};

use crate::{hyper_rpc::Transport, RpcError};

#[derive(Debug, Clone)]
pub struct ReqwestTransport {
    client: Client,
    url: String,
}

impl ReqwestTransport {
    pub fn new(url: &str) -> Self {
        let client = ClientBuilder::new()
            .tcp_nodelay(true)
            .build()
            .expect("Failed to create reqwest client");

        Self {
            client,
            url: url.to_string(),
        }
    }

    /// Create transport with minimal configuration (like Alloy uses)
    pub fn new_minimal(url: &str) -> Self {
        let client = Client::new(); // Use all defaults like Alloy

        Self {
            client,
            url: url.to_string(),
        }
    }

    /// Create transport optimized to beat Alloy performance
    pub fn new_optimized(url: &str) -> Self {
        let client = ClientBuilder::new()
            .tcp_nodelay(true) // Disable Nagle algorithm for low latency
            .pool_idle_timeout(std::time::Duration::from_secs(90)) // Keep connections longer
            .pool_max_idle_per_host(32) // More connection reuse
            .timeout(std::time::Duration::from_secs(15)) // Faster timeout than default
            .tcp_keepalive(std::time::Duration::from_secs(60)) // TCP keep-alive
            .build()
            .expect("Failed to create optimized reqwest client");

        Self {
            client,
            url: url.to_string(),
        }
    }

    /// Create ultra-fast transport to beat Alloy's 194ms
    pub fn new_ultra_fast(url: &str) -> Self {
        let client = ClientBuilder::new()
            .tcp_nodelay(true) // Disable Nagle algorithm  
            .pool_idle_timeout(std::time::Duration::from_secs(300)) // Keep connections much longer
            .pool_max_idle_per_host(64) // More aggressive connection pooling
            .timeout(std::time::Duration::from_secs(10)) // Faster timeout
            .tcp_keepalive(std::time::Duration::from_secs(30)) // More frequent keepalives
            .connect_timeout(std::time::Duration::from_secs(3)) // Faster connection timeout
            .build()
            .expect("Failed to create ultra-fast reqwest client");

        Self {
            client,
            url: url.to_string(),
        }
    }
}

#[async_trait]
impl Transport for ReqwestTransport {
    async fn hyper_execute(&self, request: String) -> Result<String, RpcError> {
        let response = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .body(request)
            .send()
            .await
            .map_err(|e| RpcError::Transport(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(RpcError::Transport(format!("HTTP error {}", response.status())));
        }

        let text = response
            .text()
            .await
            .map_err(|e| RpcError::Transport(format!("Failed to read response: {}", e)))?;

        Ok(text)
    }

    async fn hyper_execute_raw(&self, request: &'static [u8]) -> Result<Vec<u8>, RpcError> {
        let response = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .body(Bytes::from_static(request))
            .send()
            .await
            .map_err(|e| RpcError::Transport(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(RpcError::Transport(format!("HTTP error {}", response.status())));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| RpcError::Transport(format!("Failed to read response: {}", e)))?;

        Ok(bytes.to_vec())
    }

    async fn hyper_execute_bytes(&self, request: Vec<u8>) -> Result<Vec<u8>, RpcError> {
        let response = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .body(request)
            .send()
            .await
            .map_err(|e| RpcError::Transport(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(RpcError::Transport(format!("HTTP error {}", response.status())));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| RpcError::Transport(format!("Failed to read response: {}", e)))?;

        Ok(bytes.to_vec())
    }
}
