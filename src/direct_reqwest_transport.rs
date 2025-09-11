use async_trait::async_trait;
use reqwest::{Client, ClientBuilder};
use std::time::Duration;
use tracing::{debug, info};

use crate::{hyper_rpc::Transport, RpcError};

/// Direct Reqwest transport that creates fresh clients - no pooling or caching
#[derive(Debug, Clone)]
pub struct DirectReqwestTransport {
    url: String,
}

impl DirectReqwestTransport {
    pub fn new(url: &'static str) -> Self {
        info!("Creating DirectReqwestTransport with fresh clients for honest benchmarking");
        Self { url: url.to_string() }
    }

    async fn execute_request(&self, request: &[u8]) -> Result<Vec<u8>, RpcError> {
        let start = std::time::Instant::now();
        
        debug!("Creating fresh reqwest client for direct transport request");
        
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .tcp_nodelay(true)
            .pool_max_idle_per_host(0) 
            .build()
            .map_err(|e| RpcError::Transport(format!("Failed to create reqwest client: {}", e)))?;

        debug!("Making fresh HTTP request to {} (no connection reuse)", &self.url);
        
        let response = client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .body(request.to_vec())
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
        
        let duration = start.elapsed();
        debug!("Fresh HTTP request completed in {:?} (no cached connections)", duration);

        Ok(bytes.to_vec())
    }
}

#[async_trait]
impl Transport for DirectReqwestTransport {
    async fn hyper_execute(&self, request: String) -> Result<String, RpcError> {
        let response_bytes = self.execute_request(request.as_bytes()).await?;
        String::from_utf8(response_bytes)
            .map_err(|e| RpcError::Response(format!("Invalid UTF-8: {}", e)))
    }

    async fn hyper_execute_raw(&self, request: &'static [u8]) -> Result<Vec<u8>, RpcError> {
        self.execute_request(request).await
    }

    async fn hyper_execute_bytes(&self, request: Vec<u8>) -> Result<Vec<u8>, RpcError> {
        self.execute_request(&request).await
    }
}