use async_trait::async_trait;
use reqwest::Client;
use rpc::Transport;
use std::time::Duration;

pub mod cache;
pub mod rpc;
pub mod transport;

#[derive(Debug)]
pub struct HttpTransport {
    client: Client,
    url: String,
    timeout: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    #[error("Transport error: {0}")]
    Transport(String),
    #[error("Invalid response: {0}")]
    Response(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

impl HttpTransport {
    pub fn new(url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            url,
            timeout: Duration::from_secs(30),
        }
    }

    pub fn new_with_config(self) -> Self {
        Self {
            client: self.client,
            url: self.url,
            timeout: self.timeout,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn execute(&self, request: String) -> Result<String, RpcError> {
        let response = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .body(request)
            .send()
            .await
            .map_err(|e| RpcError::Transport(e.to_string()))?;

        response
            .text()
            .await
            .map_err(|e| RpcError::Transport(e.to_string()))
    }

    async fn connect(&self) -> Result<(), RpcError> {
        self.client
            .get(&self.url)
            .send()
            .await
            .map_err(|e| RpcError::Transport(e.to_string()))?;
        Ok(())
    }

    async fn execute_with_retry(&self, request: String, retry: usize) -> Result<String, RpcError> {
        let mut attempts = 0;
        
        loop {
            attempts += 1;
            
            match self.execute(request.clone()).await {
                Ok(response) => return Ok(response),
                Err(_) if attempts < retry => continue,
                Err(e) => return Err(e),
            }
        }
    }
}
