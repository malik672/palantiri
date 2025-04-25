use async_trait::async_trait;
use std::time::Duration;

use crate::{HttpTransport, RpcError};

#[async_trait]
pub trait Transport: Send + Sync + std::fmt::Debug {
    async fn execute(&self, request: String) -> Result<String, RpcError>;
    async fn connect(&self) -> Result<(), RpcError>;
}

pub struct TransportBuilder {
    url: String,
    timeout: Duration,
    max_retries: u32,
    pool_max_idle: u32,
}

impl TransportBuilder {
    /// Creates a new `TransportBuilder` with the specified URL and default configuration values.
    ///
    /// The default timeout is 10 seconds, the maximum number of retries is 3, and the maximum number of idle connections in the pool is 32.
    ///
    /// # Examples
    ///
    /// ```
    /// let builder = TransportBuilder::new("http://localhost:8080".to_string());
    /// assert_eq!(builder.max_retries, 3);
    /// ```
    pub fn new(url: String) -> Self {
        Self {
            url,
            timeout: Duration::from_secs(10),
            max_retries: 3,
            pool_max_idle: 32,
        }
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
        HttpTransport::new(self.url)
    }

    pub fn build_http_with_config(self, param: HttpTransport) -> HttpTransport {
        HttpTransport::new_with_config(param)
    }
}
