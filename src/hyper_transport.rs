use std::time::Duration;
use std::sync::{Arc, OnceLock};
use std::collections::VecDeque;
use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::header::HeaderValue;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::rt::TokioExecutor;
use tokio::sync::{Mutex, Semaphore, oneshot};
use tracing::{debug, info, warn};

use crate::hyper_rpc::Transport;
use crate::RpcError;

const DURATION_TIMEOUT: Duration = std::time::Duration::from_secs(30);
const CONTENT_TYPE_JSON: HeaderValue = HeaderValue::from_static("application/json");
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_IDLE_POOL: usize = 200;
const TCP_KEEPALIVE: Duration = Duration::from_secs(60);
const HAPPY_EYEBALLS_TIMEOUT: Duration = Duration::from_millis(300);
const MAX_CONCURRENT_REQUESTS: usize = 50;
const PIPELINE_BUFFER_SIZE: usize = 32;

type HttpClient = hyper_util::client::legacy::Client<
    HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
    http_body_util::Full<::hyper::body::Bytes>,
>;

static CLIENT_POOL: OnceLock<Arc<HttpClient>> = OnceLock::new();

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
    fallback_urls: Vec<&'static str>,
    pipeline: Arc<RequestPipeline>,
}

impl HyperTransport {
    pub fn new(url: &'static str) -> Self {
        let client = CLIENT_POOL.get_or_init(|| {
            debug!("Creating shared HTTP client with pipelining support");

            let http_executor = TokioExecutor::new();

            let mut http_connector = hyper_util::client::legacy::connect::HttpConnector::new();
            http_connector.set_connect_timeout(Some(CONNECTION_TIMEOUT));
            http_connector.set_keepalive(Some(TCP_KEEPALIVE));
            http_connector.set_nodelay(true);
            http_connector.set_happy_eyeballs_timeout(Some(HAPPY_EYEBALLS_TIMEOUT));
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
                .pool_idle_timeout(DURATION_TIMEOUT)
                .pool_max_idle_per_host(MAX_IDLE_POOL)
                .retry_canceled_requests(false)
                .build(https_connector);

            info!("Shared HTTP client with pipelining created successfully");
            Arc::new(client)
        });

        let pipeline = Arc::new(RequestPipeline::new());
        
        let transport = Self {
            client: client.clone(),
            primary_url: url,
            fallback_urls: Vec::new(),
            pipeline: pipeline.clone(),
        };
        
        let pipeline_clone = pipeline.clone();
        let transport_clone = transport.clone();
        tokio::spawn(async move {
            RequestPipeline::start_processing(pipeline_clone, transport_clone).await;
        });
        
        transport
    }

    pub fn new_with_fallbacks(primary_url: &'static str, fallback_urls: Vec<&'static str>) -> Self {
        let client = CLIENT_POOL.get_or_init(|| {
            debug!("Creating shared HTTP client with fallbacks and pipelining");

            let http_executor = TokioExecutor::new();

            let mut http_connector = hyper_util::client::legacy::connect::HttpConnector::new();
            http_connector.set_connect_timeout(Some(CONNECTION_TIMEOUT));
            http_connector.set_keepalive(Some(TCP_KEEPALIVE));
            http_connector.set_nodelay(true);
            http_connector.set_happy_eyeballs_timeout(Some(HAPPY_EYEBALLS_TIMEOUT));
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
                .pool_idle_timeout(DURATION_TIMEOUT)
                .pool_max_idle_per_host(MAX_IDLE_POOL)
                .retry_canceled_requests(false)
                .build(https_connector);

            info!("Shared HTTP client with fallbacks and pipelining created successfully");
            Arc::new(client)
        });

        let pipeline = Arc::new(RequestPipeline::new());
        
        let transport = Self {
            client: client.clone(),
            primary_url,
            fallback_urls,
            pipeline: pipeline.clone(),
        };
        
        let pipeline_clone = pipeline.clone();
        let transport_clone = transport.clone();
        tokio::spawn(async move {
            RequestPipeline::start_processing(pipeline_clone, transport_clone).await;
        });
        
        transport
    }

    pub async fn execute_single_request(&self, request: &[u8]) -> Result<Vec<u8>, RpcError> {
        let req = hyper::Request::builder()
            .method(hyper::Method::POST)
            .uri(self.primary_url)
            .header(hyper::header::CONTENT_TYPE, CONTENT_TYPE_JSON)
            .header(hyper::header::CONNECTION, "keep-alive")
            .header("User-Agent", "palantiri/0.1.0")
            .body(http_body_util::Full::new(Bytes::from(request.to_vec())))
            .map_err(|e| RpcError::Transport(format!("Failed to build request: {}", e)))?;

        let response = self
            .client
            .request(req)
            .await
            .map_err(|e| RpcError::Transport(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(RpcError::Transport(format!("HTTP error {}", response.status())));
        }

        let body_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e| RpcError::Transport(e.to_string()))?
            .to_bytes();

        Ok(body_bytes.to_vec())
    }

    async fn try_request(&self, url: &str, request: &[u8]) -> Result<Vec<u8>, RpcError> {
        let req = hyper::Request::builder()
            .method(hyper::Method::POST)
            .uri(url)
            .header(hyper::header::CONTENT_TYPE, CONTENT_TYPE_JSON)
            .header(hyper::header::CONNECTION, "keep-alive")
            .body(http_body_util::Full::new(Bytes::from(request.to_vec())))
            .map_err(|e| RpcError::Transport(format!("Failed to build request: {}", e)))?;

        let response = self
            .client
            .request(req)
            .await
            .map_err(|e| RpcError::Transport(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(RpcError::Transport(format!("HTTP error {}", response.status())));
        }

        let body_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e| RpcError::Transport(e.to_string()))?
            .to_bytes();

        Ok(body_bytes.to_vec())
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

        let body_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e| RpcError::Transport(e.to_string()))?
            .to_bytes();

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

        // Use optimized body collection
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e| RpcError::Transport(e.to_string()))?
            .to_bytes();

        Ok(body_bytes.into())
    }

    async fn hyper_execute_bytes(&self, request: Vec<u8>) -> Result<Vec<u8>, RpcError> {
        self.pipeline.enqueue_request(request).await
    }
}

impl HyperTransport {
    pub async fn hyper_execute_bytes_batch(&self, requests: Vec<Vec<u8>>) -> Result<Vec<Result<Vec<u8>, RpcError>>, RpcError> {
        let mut handles = Vec::new();
        
        for request in requests {
            let pipeline = self.pipeline.clone();
            let handle = tokio::spawn(async move {
                pipeline.enqueue_request(request).await
            });
            handles.push(handle);
        }
        
        let results = futures::future::join_all(handles).await;
        let mut responses = Vec::new();
        
        for result in results {
            match result {
                Ok(response) => responses.push(response),
                Err(e) => responses.push(Err(RpcError::Transport(format!("Task join error: {}", e)))),
            }
        }
        
        Ok(responses)
    }
}
