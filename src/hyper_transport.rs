use std::{
    collections::VecDeque,
    sync::{Arc, OnceLock},
    time::Duration,
};

use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::header::HeaderValue;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::rt::TokioExecutor;
use tokio::sync::{oneshot, Mutex, Semaphore};
use tracing::{debug, info};

use crate::{hyper_rpc::Transport, RpcError};

const CONTENT_TYPE_JSON: HeaderValue = HeaderValue::from_static("application/json");
const MAX_CONCURRENT_REQUESTS: usize = 100;

type HttpClient = hyper_util::client::legacy::Client<
    HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
    http_body_util::Full<::hyper::body::Bytes>,
>;

static CLIENT_POOL: OnceLock<Arc<HttpClient>> = OnceLock::new();
static BENCHMARK_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[derive(Debug)]
struct PipelineRequest {
    request: Vec<u8>,
    #[allow(dead_code)] // Used in debug output
    response_sender: oneshot::Sender<Result<Vec<u8>, RpcError>>,
}

#[derive(Debug)]
pub struct RequestPipeline {
    queue: Arc<Mutex<VecDeque<PipelineRequest>>>,
    semaphore: Arc<Semaphore>,
}

impl Default for RequestPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestPipeline {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS)),
        }
    }

    pub async fn enqueue_request(&self, request: Vec<u8>) -> Result<Vec<u8>, RpcError> {
        let (tx, rx) = oneshot::channel();

        {
            let mut queue = self.queue.lock().await;
            queue.push_back(PipelineRequest {
                request,
                response_sender: tx,
            });
        }

        rx.await
            .map_err(|_| RpcError::Transport("Pipeline request cancelled".to_string()))?
    }

    pub async fn start_processing(pipeline: Arc<RequestPipeline>, transport: HyperTransport) {
        let queue = pipeline.queue.clone();
        let semaphore = pipeline.semaphore.clone();

        loop {
            let permit = semaphore.acquire().await.unwrap();

            let request = {
                let mut queue = queue.lock().await;
                queue.pop_front()
            };

            if let Some(pipeline_req) = request {
                let transport = transport.clone();
                // Execute synchronously instead of spawning to avoid lifetime issues
                let result = transport.execute_single_request(&pipeline_req.request).await;
                let _ = pipeline_req.response_sender.send(result);
                drop(permit);
            } else {
                drop(permit);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct HyperTransport {
    client: Arc<HttpClient>,
    primary_url: &'static str,
    pipeline: Arc<RequestPipeline>,
}

impl HyperTransport {
    /// Create minimal HTTP client matching Alloy's configuration for maximum performance
    pub fn new_minimal_like_alloy(url: &'static str) -> Self {
        let client = CLIENT_POOL.get_or_init(|| {
            debug!("Creating minimal Alloy-style HTTP client");

            let http_executor = TokioExecutor::new();
            
            // Optimized HTTP connector - match reqwest defaults for best performance
            let mut http_connector = hyper_util::client::legacy::connect::HttpConnector::new();
            http_connector.set_nodelay(true); // Disable Nagle algorithm for low latency
            http_connector.set_keepalive(Some(Duration::from_secs(30))); // Match reqwest default
            http_connector.set_connect_timeout(Some(Duration::from_millis(30_000))); // 30s like reqwest
            
            // Minimal HTTPS connector - match Alloy's approach
            let https_connector = HttpsConnectorBuilder::new()
                .with_provider_and_webpki_roots(rustls::crypto::aws_lc_rs::default_provider())
                .expect("Failed to load native root certificates")
                .https_or_http()
                .enable_http1()
                .enable_http2()
                .wrap_connector(http_connector);
            
            // Optimized client builder - balanced settings for performance
            let client = hyper_util::client::legacy::Client::builder(http_executor)
                .pool_idle_timeout(Duration::from_secs(30)) // Match reqwest defaults
                .pool_max_idle_per_host(32) // Conservative pool size
                .build(https_connector);
            
            info!("Minimal Alloy-style HTTP client created");
            Arc::new(client)
        });

        // No request pipeline for minimal approach
        Self {
            client: client.clone(),
            primary_url: url,
            pipeline: Arc::new(RequestPipeline::new()),
        }
    }

    pub fn new_ultra_fast(url: &'static str) -> Self {
        let client = CLIENT_POOL.get_or_init(|| {
            debug!("Creating ultra-fast HTTP client to beat Alloy");

            let http_executor = TokioExecutor::new();
            
            // Ultra-optimized HTTP connector - aggressive settings
            let mut http_connector = hyper_util::client::legacy::connect::HttpConnector::new();
            http_connector.set_nodelay(true); // Disable Nagle
            http_connector.set_keepalive(Some(Duration::from_secs(90))); // Longer keepalive
            http_connector.set_connect_timeout(Some(Duration::from_millis(2000))); // Faster timeout
            http_connector.set_happy_eyeballs_timeout(Some(Duration::from_millis(50))); // IPv6 fallback
            
            // Ultra-fast HTTPS connector 
            let https_connector = HttpsConnectorBuilder::new()
                .with_provider_and_webpki_roots(rustls::crypto::aws_lc_rs::default_provider())
                .expect("Failed to load native root certificates")
                .https_or_http()
                .enable_http1()
                .enable_http2() // Critical for performance
                .wrap_connector(http_connector);
            
            // Aggressive connection pool settings
            let client = hyper_util::client::legacy::Client::builder(http_executor)
                .pool_idle_timeout(Duration::from_secs(120)) // Keep connections alive longer
                .pool_max_idle_per_host(64) // More pooled connections
                .build(https_connector);
                
            info!("Ultra-fast HTTP client created to beat Alloy");
            Arc::new(client)
        });
        
        Self {
            client: client.clone(),
            primary_url: url,
            pipeline: Arc::new(RequestPipeline::new()),
        }
    }

    /// Create a performant client for benchmarking - allows some connection reuse
    /// Similar to what Alloy does for realistic comparison
    pub fn new_benchmark_realistic(url: &'static str) -> Self {
        debug!("Creating realistic benchmark HTTP client (minimal connection reuse)");

        let http_executor = TokioExecutor::new();
        
        let mut http_connector = hyper_util::client::legacy::connect::HttpConnector::new();
        http_connector.set_nodelay(true);
        http_connector.set_keepalive(Some(Duration::from_secs(30))); // Allow some reuse like Alloy
        http_connector.set_connect_timeout(Some(Duration::from_secs(10)));
        http_connector.set_happy_eyeballs_timeout(Some(Duration::from_millis(300)));
        
        let https_connector = HttpsConnectorBuilder::new()
            .with_provider_and_webpki_roots(rustls::crypto::aws_lc_rs::default_provider())
            .expect("Failed to load native root certificates")
            .https_or_http()
            .enable_http1()
            .enable_http2() // Enable HTTP/2 like Alloy
            .wrap_connector(http_connector);
        
        // Minimal pooling client to match Alloy's approach
        let client = hyper_util::client::legacy::Client::builder(http_executor)
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10) // Minimal pooling
            .build(https_connector);

        info!("Created realistic benchmark client with minimal connection reuse");

        Self {
            client: Arc::new(client),
            primary_url: url,
            pipeline: Arc::new(RequestPipeline::new()),
        }
    }

    pub fn new(url: &'static str) -> Self {
        let client = CLIENT_POOL.get_or_init(|| {
            debug!("Creating shared HTTP client with pipelining support");

            let http_executor = TokioExecutor::new();

            let mut http_connector = hyper_util::client::legacy::connect::HttpConnector::new();
            http_connector.set_connect_timeout(Some(Duration::from_millis(2000)));
            http_connector.set_keepalive(Some(Duration::from_secs(90)));
            http_connector.set_nodelay(true);
            http_connector.set_happy_eyeballs_timeout(Some(Duration::from_millis(100)));
            http_connector.set_reuse_address(true);
            http_connector.enforce_http(false);

            let https_connector = HttpsConnectorBuilder::new()
                .with_provider_and_webpki_roots(rustls::crypto::aws_lc_rs::default_provider())
                .expect("Failed to load native root certificates")
                .https_or_http()
                .enable_http1()
                .enable_http2()
                .wrap_connector(http_connector);

            let client = hyper_util::client::legacy::Client::builder(http_executor)
                .pool_idle_timeout(Duration::from_secs(90))
                .pool_max_idle_per_host(100)
                .retry_canceled_requests(false)
                .build(https_connector);

            info!("Shared HTTP client with pipelining created successfully");
            Arc::new(client)
        });

        let pipeline = Arc::new(RequestPipeline::new());

        Self {
            client: client.clone(),
            primary_url: url,
            pipeline: pipeline.clone(),
        }
    }

    pub fn new_with_fallbacks(
        primary_url: &'static str,
        _fallback_urls: Vec<&'static str>,
    ) -> Self {
        let client = CLIENT_POOL.get_or_init(|| {
            debug!("Creating shared HTTP client with fallbacks and pipelining");

            let http_executor = TokioExecutor::new();

            let mut http_connector = hyper_util::client::legacy::connect::HttpConnector::new();
            http_connector.set_connect_timeout(Some(Duration::from_millis(2000)));
            http_connector.set_keepalive(Some(Duration::from_secs(90)));
            http_connector.set_nodelay(true);
            http_connector.set_happy_eyeballs_timeout(Some(Duration::from_millis(100)));
            http_connector.set_reuse_address(true);
            http_connector.enforce_http(false);

            let https_connector = HttpsConnectorBuilder::new()
                .with_provider_and_webpki_roots(rustls::crypto::aws_lc_rs::default_provider())
                .expect("Failed to load native root certificates")
                .https_or_http()
                .enable_http1()
                .enable_http2()
                .wrap_connector(http_connector);

            let client = hyper_util::client::legacy::Client::builder(http_executor)
                .pool_idle_timeout(Duration::from_secs(90))
                .pool_max_idle_per_host(100)
                .retry_canceled_requests(false)
                .build(https_connector);

            info!("Shared HTTP client with fallbacks and pipelining created successfully");
            Arc::new(client)
        });

        let pipeline = Arc::new(RequestPipeline::new());

        Self {
            client: client.clone(),
            primary_url,
            pipeline: pipeline.clone(),
        }
    }

    pub async fn execute_single_request(&self, request: &[u8]) -> Result<Vec<u8>, RpcError> {
        let start = std::time::Instant::now();
        let body = http_body_util::Full::new(Bytes::copy_from_slice(request));

        let req = hyper::Request::builder()
            .method(hyper::Method::POST)
            .uri(self.primary_url)
            .header(hyper::header::CONTENT_TYPE, CONTENT_TYPE_JSON)
            .body(body)
            .map_err(|e| RpcError::Transport(format!("Failed to build request: {}", e)))?;

        debug!("Making HTTP request to {}", self.primary_url);
        let response = self
            .client
            .request(req)
            .await
            .map_err(|e| RpcError::Transport(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(RpcError::Transport(format!("HTTP error {}", response.status())));
        }

        let body = response.into_body();
        let body_bytes =
            body.collect().await.map_err(|e| RpcError::Transport(e.to_string()))?.to_bytes();
        
        let duration = start.elapsed();
        debug!("HTTP request completed in {:?}, response size: {} bytes", duration, body_bytes.len());

        Ok(body_bytes.into())
    }
}

#[async_trait]
impl Transport for HyperTransport {
    async fn hyper_execute(&self, request: String) -> Result<String, RpcError> {
        let url = self.primary_url;

        let reqs = request.as_bytes().to_owned().into();

        let req = hyper::Request::builder()
            .method(hyper::Method::POST)
            .uri(url)
            .header("Content-Type", CONTENT_TYPE_JSON)
            .body(http_body_util::Full::new(reqs))
            .expect("Failed to build request");

        let response = self
            .client
            .request(req)
            .await
            .map_err(|e| RpcError::Transport(format!("Request failed: {}", e)))?;

        let body = response.into_body();
        let body_bytes =
            body.collect().await.map_err(|e| RpcError::Transport(e.to_string()))?.to_bytes();

        String::from_utf8(body_bytes.to_vec())
            .map_err(|e| RpcError::Response(format!("Invalid UTF-8 in response: {}", e)))
    }

    async fn hyper_execute_raw(&self, request: &'static [u8]) -> Result<Vec<u8>, RpcError> {
        let req = hyper::Request::builder()
            .method(hyper::Method::POST)
            .uri(self.primary_url)
            .header(hyper::header::CONTENT_TYPE, CONTENT_TYPE_JSON)
            .header(hyper::header::CONNECTION, "keep-alive")
            .body(http_body_util::Full::new(Bytes::from_static(request)))
            .map_err(|e| RpcError::Transport(format!("Failed to build request: {}", e)))?;

        let response = self
            .client
            .request(req)
            .await
            .map_err(|e| RpcError::Transport(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(RpcError::Transport(format!("HTTP error {}", response.status())));
        }

        // Direct body collection without intermediate conversion
        let body = response.into_body();
        let body_bytes =
            body.collect().await.map_err(|e| RpcError::Transport(e.to_string()))?.to_bytes();

        Ok(body_bytes.into())
    }

    async fn hyper_execute_bytes(&self, request: Vec<u8>) -> Result<Vec<u8>, RpcError> {
        self.execute_single_request(&request).await
    }
}

impl HyperTransport {
    /// Direct execution bypassing pipeline for minimal latency
    pub async fn execute_direct(&self, request: &[u8]) -> Result<Vec<u8>, RpcError> {
        self.execute_single_request(request).await
    }

    /// Ultra-fast batch execution using HTTP/2 multiplexing
    pub async fn execute_batch_optimized(
        &self,
        requests: Vec<Vec<u8>>,
    ) -> Result<Vec<Result<Vec<u8>, RpcError>>, RpcError> {
        if requests.is_empty() {
            return Ok(Vec::new());
        }

        if requests.len() == 1 {
            let result = self.execute_single_request(&requests[0]).await;
            return Ok(vec![result]);
        }

        // Use HTTP/2 multiplexing for concurrent requests
        let futures = requests.into_iter().map(|req| {
            let client = self.client.clone();
            let url = self.primary_url;
            
            async move {
                let body = http_body_util::Full::new(bytes::Bytes::from(req));
                let request = hyper::Request::builder()
                    .method(hyper::Method::POST)
                    .uri(url)
                    .header(hyper::header::CONTENT_TYPE, CONTENT_TYPE_JSON)
                    .body(body)
                    .map_err(|e| RpcError::Transport(format!("Failed to build request: {}", e)))?;

                let response = client
                    .request(request)
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

                Ok(body_bytes.to_vec())
            }
        });

        // Execute all requests concurrently using HTTP/2 multiplexing
        let results = futures::future::join_all(futures).await;
        Ok(results)
    }

    pub async fn hyper_execute_bytes_batch(
        &self,
        requests: Vec<Vec<u8>>,
    ) -> Result<Vec<Result<Vec<u8>, RpcError>>, RpcError> {
        let mut handles = Vec::new();

        for request in requests {
            let pipeline = self.pipeline.clone();
            let handle = tokio::spawn(async move { pipeline.enqueue_request(request).await });
            handles.push(handle);
        }

        let results = futures::future::join_all(handles).await;
        let mut responses = Vec::new();

        for result in results {
            match result {
                Ok(response) => responses.push(response),
                Err(e) => {
                    responses.push(Err(RpcError::Transport(format!("Task join error: {}", e))))
                },
            }
        }

        Ok(responses)
    }
}
