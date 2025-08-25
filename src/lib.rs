use reqwest::Client;
use std::time::Duration;

pub mod transport;
pub mod parser;
pub mod hyper_transport;
pub mod hyper_rpc;



#[derive(Debug)]
pub struct HttpTransport {
    client: Client,
    urls:  &'static str,
    current_url: usize,
    timeout: Duration,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum RpcError {
    #[error("Transport error: {0}")]
    Transport(String),
    #[error("Invalid response: {0}")]
    Response(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

impl HttpTransport {
    pub fn new(url: &'static str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(60))
            .tcp_keepalive(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            urls: url,
            current_url: 0,
            timeout: Duration::from_secs(30),
        }
    }

    pub fn new_with_config(self) -> Self {
        Self {
            client: self.client,
            urls: self.urls,
            current_url: self.current_url,
            timeout: self.timeout,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}


