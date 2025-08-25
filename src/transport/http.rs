use async_trait::async_trait;
use std::time::Duration;

use crate::{hyper_transport::HyperTransport, HttpTransport, RpcError};

#[async_trait]
pub trait Transport: Send + Sync + std::fmt::Debug {
    async fn execute(&self, request: String) -> Result<String, RpcError>;
    async fn connect(&self) -> Result<(), RpcError>;
}

pub struct TransportBuilder {
    urls: Vec<&'static str>,
    timeout: Duration,
    max_retries: u32,
    pool_max_idle: u32,
}

impl TransportBuilder {
    pub fn new(url: &'static str) -> Self {
        Self {
            urls: vec![url],
            timeout: Duration::from_secs(10),
            max_retries: 3,
            pool_max_idle: 32,
        }
    }

    pub fn with_fallbacks(mut self, additional_urls: Vec<&'static str>) -> Self {
        self.urls.extend(additional_urls);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    pub fn pool_max_idle(mut self, max_idle: u32) -> Self {
        self.pool_max_idle = max_idle;
        self
    }

    pub fn build_http(self) -> HttpTransport {
        HttpTransport::new(self.urls[0])
    }

    pub fn build_http_hyper(self) -> HyperTransport {
        if self.urls.len() > 1 {
            HyperTransport::new_with_fallbacks(self.urls[0], self.urls[1..].to_vec())
        } else {
            HyperTransport::new(self.urls[0])
        }
    }

    pub fn build_http_with_config(self, param: HttpTransport) -> HttpTransport {
        HttpTransport::new_with_config(param)
    }
}
