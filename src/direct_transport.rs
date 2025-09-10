use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::rt::TokioExecutor;
use std::time::Duration;
use tracing::{debug, info};

use crate::{hyper_rpc::Transport, RpcError};

const CONTENT_TYPE_JSON: &str = "application/json";

/// Direct HTTP transport that creates fresh connections - no pooling or caching
#[derive(Debug, Clone)]
pub struct DirectTransport {
    url: String,
}

impl DirectTransport {
    pub fn new(url: &'static str) -> Self {
        info!("Creating DirectTransport with no connection pooling for honest benchmarking");
        Self { url: url.to_string() }
    }

    async fn execute_request(&self, request: &[u8]) -> Result<Vec<u8>, RpcError> {
        let start = std::time::Instant::now();
        
        debug!("Creating fresh HTTP client for direct transport request");
        
        let http_executor = TokioExecutor::new();
        
        // Use minimal connector settings - match what works in the existing codebase
        let mut http_connector = hyper_util::client::legacy::connect::HttpConnector::new();
        http_connector.set_nodelay(true);
        http_connector.set_keepalive(Some(Duration::from_secs(30)));
        http_connector.set_connect_timeout(Some(Duration::from_millis(30_000)));
        
        let https_connector = HttpsConnectorBuilder::new()
            .with_provider_and_webpki_roots(rustls::crypto::aws_lc_rs::default_provider())
            .expect("Failed to load native root certificates")
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .wrap_connector(http_connector);
        
        // Build fresh client each time with minimal settings
        let client = hyper_util::client::legacy::Client::builder(http_executor)
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(0) // NO connection reuse at all for honest benchmarking
            .build(https_connector);
        
        let body = Full::new(Bytes::copy_from_slice(request));
        
        let req = hyper::Request::builder()
            .method(hyper::Method::POST)
            .uri(&self.url)
            .header("Content-Type", CONTENT_TYPE_JSON)
            .body(body)
            .map_err(|e| RpcError::Transport(format!("Failed to build request: {}", e)))?;

        debug!("Making fresh HTTP request to {} (no connection reuse)", &self.url);
        let response = client
            .request(req)
            .await
            .map_err(|e| RpcError::Transport(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(RpcError::Transport(format!("HTTP error {}", response.status())));
        }

        let body = response.into_body();
        let body_bytes = body
            .collect()
            .await
            .map_err(|e| RpcError::Transport(e.to_string()))?
            .to_bytes();
        
        let duration = start.elapsed();
        debug!("Fresh HTTP request completed in {:?} (no cached connections)", duration);

        Ok(body_bytes.into())
    }
}

#[async_trait]
impl Transport for DirectTransport {
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