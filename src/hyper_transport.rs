use std::time::{Duration, Instant};

use async_trait::async_trait;
use hyper::{Client, Method, Request, Body};
use hyper_tls::HttpsConnector;
use tracing::{debug, error, info};

use crate::RpcError;
use crate::hyper_rpc::Transport;

#[derive(Debug)]
pub struct HyperTransport {
    client: Client<HttpsConnector<hyper::client::HttpConnector>>,
    url: String,
}

impl HyperTransport {
    pub fn new(url: String) -> Self {
        info!("Creating new HyperTransport instance");

        // Start timing the entire operation
        let start = Instant::now();

        // Initialize HttpsConnector
        debug!("Initializing HttpsConnector");
        let connector_start = Instant::now();
        let https = HttpsConnector::new();
        let connector_duration = connector_start.elapsed();
        debug!(
            duration_ms = connector_duration.as_millis(),
            "HttpsConnector created successfully"
        );

        // Build Hyper client
        debug!("Building Hyper client with pool settings");
        let client_start = Instant::now();
        let client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(32)
            .build::<_, Body>(https);
        let client_duration = client_start.elapsed();
        debug!(
            duration_ms = client_duration.as_millis(),
            pool_idle_timeout_secs = 60,
            max_idle_per_host = 32,
            "Hyper client built successfully"
        );

        let instance = Self { client, url };
        let total_duration = start.elapsed();
        info!(
            url = %instance.url,
            total_duration_ms = total_duration.as_millis(),
            "HyperTransport instance created"
        );
        instance
    }
}

#[async_trait]
impl Transport for HyperTransport {
     async fn hyper_execute(&self, request: String) -> Result<String, RpcError> {
       
    
        let req = Request::builder()
            .method(Method::POST)
            .uri(self.url.as_str())
            .header("Content-Type", "application/json")
            .body(hyper::Body::from(request))
            .map_err(|e| RpcError::Transport(format!("Failed to build request: {}", e)))?;
    
        let response = self.client
            .request(req)
            .await
            .map_err(|e| RpcError::Transport(format!("Request failed: {}", e)))?;
    
        let body_bytes = hyper::body::to_bytes(response.into_body())
            .await
            .map_err(|e| RpcError::Transport(format!("Failed to read response body: {}", e)))?;
    
        String::from_utf8(body_bytes.to_vec())
            .map_err(|e| RpcError::Response(format!("Invalid UTF-8 in response: {}", e)))
    }
    
    
     async fn hyper_execute_raw(&self, request: String) -> Result<Vec<u8>, RpcError> {
    
        let req = Request::builder()
            .method(Method::POST)
            .uri(self.url.as_str())
            .header("Content-Type", "application/json")
            .body(hyper::Body::from(request))
            .map_err(|e| RpcError::Transport(format!("Failed to build request: {}", e)))?;
    
        let response = self.client
            .request(req)
            .await
            .map_err(|e| RpcError::Transport(format!("Request failed: {}", e)))?;
    
        let body_bytes = hyper::body::to_bytes(response.into_body())
            .await
            .map_err(|e| RpcError::Transport(format!("Failed to read response body: {}", e)))?;
    
            Ok(body_bytes.to_vec())
    }
    
}