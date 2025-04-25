use async_trait::async_trait;
use reqwest::Client;
use rpc::Transport;
use std::time::Duration;

pub mod rpc;
pub mod transport;
pub mod parser;
use dotenv;
use std::env;

#[derive(Debug)]
pub struct HttpTransport {
    client: Client,
    urls: Vec<String>,
    current_url: usize,
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
    /// Creates a new `HttpTransport` with a single URL and a preconfigured HTTP client.
    ///
    /// The client is initialized with a 10-second timeout, 10-second pool idle timeout, and 60-second TCP keepalive. Panics if the HTTP client cannot be created.
    ///
    /// # Examples
    ///
    /// ```
    /// let transport = HttpTransport::new("https://example.com/api".to_string());
    /// ```
    pub fn new(url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(10))
            // .http2_prior_knowledge()
            .tcp_keepalive(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            urls: vec![url],
            current_url: 0,
            timeout: Duration::from_secs(10),
        }
    }

    /// Creates a new `HttpTransport` instance using environment variables for configuration.
    ///
    /// Loads the primary URL from the `PRIMARY_URL` environment variable (required), optional fallback URLs from `FALLBACK_URL_1` and `FALLBACK_URL_2`, and the transport timeout from `TRANSPORT_TIMEOUT` (defaults to 10 seconds if unset). Initializes the HTTP client with these settings.
    ///
    /// # Panics
    ///
    /// Panics if `PRIMARY_URL` is not set or if the HTTP client cannot be created.
    ///
    /// # Examples
    ///
    /// ```
    /// std::env::set_var("PRIMARY_URL", "https://example.com/api");
    /// let transport = HttpTransport::new_from_env();
    /// assert_eq!(transport.urls[0], "https://example.com/api");
    /// ```
    pub fn new_from_env() -> Self {
        dotenv::dotenv().ok();

        let primary_url = env::var("PRIMARY_URL").expect("PRIMARY_URL must be set in environment");

        let mut urls = vec![primary_url];
        if let Ok(fallback1) = env::var("FALLBACK_URL_1") {
            urls.push(fallback1);
        }
        if let Ok(fallback2) = env::var("FALLBACK_URL_2") {
            urls.push(fallback2);
        }

        let transport_timeout = env::var("TRANSPORT_TIMEOUT")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .unwrap_or(10);

        let client = Client::builder()
            .timeout(Duration::from_secs(transport_timeout))
            .pool_idle_timeout(Duration::from_secs(10))
            .tcp_keepalive(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            urls,
            current_url: 0,
            timeout: Duration::from_secs(transport_timeout),
        }
    }

    pub fn with_fallback_urls(mut self, urls: Vec<String>) -> Self {
        self.urls.extend(urls);
        self
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

#[async_trait]
impl Transport for HttpTransport {
    async fn execute(&self, request: String) -> Result<String, RpcError> {
        let url = &self.urls[self.current_url];

        let response = self
            .client
            .post(url)
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

    async fn execute_raw(&self, request: String) -> Result<Vec<u8>, RpcError> {
        let url = &self.urls[self.current_url];

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .body(request)
            .send()
            .await
            .map_err(|e| RpcError::Transport(e.to_string()))?;

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| RpcError::Transport(e.to_string()))
    }
}
