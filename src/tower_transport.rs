use std::{
    sync::Arc,
    time::{Duration, Instant},
    task::{Context, Poll},
    pin::Pin,
    future::Future,
};

use async_trait::async_trait;
use bytes::Bytes;
use http::Request;
use http_body_util::{BodyExt, Full};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::rt::TokioExecutor;
use tower::{
    limit::ConcurrencyLimit,
    timeout::Timeout,
    Service, ServiceBuilder, ServiceExt, Layer,
};
use tracing::{debug, info, instrument};

use crate::{hyper_rpc::Transport, RpcError};

type HttpClient = hyper_util::client::legacy::Client<
    HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
    Full<Bytes>,
>;

#[derive(Clone)]
pub struct MetricsMiddleware<S> {
    inner: S,
}

impl<S> MetricsMiddleware<S> {
    pub fn new(service: S) -> Self {
        Self { inner: service }
    }
}

impl<S, B> Service<Request<B>> for MetricsMiddleware<S>
where
    S: Service<Request<B>>,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let start = Instant::now();
        let method = req.method().clone();
        let uri = req.uri().clone();
        
        let future = self.inner.call(req);
        
        Box::pin(async move {
            let result = future.await;
            let duration = start.elapsed();
            
            match &result {
                Ok(_) => {
                    debug!("Request succeeded: {} {} in {:?}", method, uri, duration);
                }
                Err(_) => {
                    debug!("Request failed: {} {} in {:?}", method, uri, duration);
                }
            }
            
            result
        })
    }
}

pub struct MetricsMiddlewareLayer;

impl Default for MetricsMiddlewareLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsMiddlewareLayer {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for MetricsMiddlewareLayer {
    type Service = MetricsMiddleware<S>;

    fn layer(&self, service: S) -> Self::Service {
        MetricsMiddleware::new(service)
    }
}

#[derive(Clone)]
pub struct CircuitBreaker<S> {
    inner: S,
    failure_count: Arc<std::sync::atomic::AtomicU32>,
    last_failure: Arc<std::sync::Mutex<Option<Instant>>>,
    threshold: u32,
    timeout: Duration,
}

impl<S> CircuitBreaker<S> {
    pub fn new(service: S, threshold: u32, timeout: Duration) -> Self {
        Self {
            inner: service,
            failure_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            last_failure: Arc::new(std::sync::Mutex::new(None)),
            threshold,
            timeout,
        }
    }

    fn is_circuit_open(&self) -> bool {
        let count = self.failure_count.load(std::sync::atomic::Ordering::Relaxed);
        if count >= self.threshold {
            if let Ok(last_failure) = self.last_failure.lock() {
                if let Some(time) = *last_failure {
                    return time.elapsed() < self.timeout;
                }
            }
        }
        false
    }
}

impl<S, B> Service<Request<B>> for CircuitBreaker<S>
where
    S: Service<Request<B>>,
    S::Error: std::fmt::Debug,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        if self.is_circuit_open() {
            debug!("Circuit breaker is open, failing fast");
            return Box::pin(async move {
                std::future::pending::<Result<Self::Response, Self::Error>>().await
            });
        }

        let future = self.inner.call(req);
        let failure_count = self.failure_count.clone();
        let last_failure = self.last_failure.clone();

        Box::pin(async move {
            match future.await {
                Ok(response) => {
                    failure_count.store(0, std::sync::atomic::Ordering::Relaxed);
                    if let Ok(mut last_failure) = last_failure.lock() {
                        *last_failure = None;
                    }
                    Ok(response)
                }
                Err(error) => {
                    failure_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if let Ok(mut last_failure) = last_failure.lock() {
                        *last_failure = Some(Instant::now());
                    }
                    debug!("Request failed, circuit breaker failure count: {}", 
                           failure_count.load(std::sync::atomic::Ordering::Relaxed));
                    Err(error)
                }
            }
        })
    }
}

pub struct CircuitBreakerLayer {
    threshold: u32,
    timeout: Duration,
}

impl CircuitBreakerLayer {
    pub fn new(threshold: u32, timeout: Duration) -> Self {
        Self { threshold, timeout }
    }
}

impl<S> Layer<S> for CircuitBreakerLayer {
    type Service = CircuitBreaker<S>;

    fn layer(&self, service: S) -> Self::Service {
        CircuitBreaker::new(service, self.threshold, self.timeout)
    }
}

#[derive(Debug, Clone)]
pub struct TowerTransport {
    client: Arc<ConcurrencyLimit<Timeout<HttpClient>>>,
    url: &'static str,
}

impl TowerTransport {
    pub fn new(url: &'static str) -> Self {
        info!("Creating Tower-based HTTP client");

        let http_executor = TokioExecutor::new();
        
        let mut http_connector = hyper_util::client::legacy::connect::HttpConnector::new();
        http_connector.set_nodelay(true);
        http_connector.set_keepalive(Some(Duration::from_secs(30)));
        http_connector.set_connect_timeout(Some(Duration::from_secs(10)));
        
        let https_connector = HttpsConnectorBuilder::new()
            .with_provider_and_webpki_roots(rustls::crypto::aws_lc_rs::default_provider())
            .expect("Failed to load root certificates")
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .wrap_connector(http_connector);

        let base_client = hyper_util::client::legacy::Client::builder(http_executor)
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(16)
            .build(https_connector);

        let tower_client = ServiceBuilder::new()
            .concurrency_limit(50)
            .timeout(Duration::from_secs(30))
            .service(base_client);

        info!("Tower HTTP client created");

        Self {
            client: Arc::new(tower_client),
            url,
        }
    }

    pub fn new_optimized(url: &'static str) -> Self {
        debug!("Creating optimized Tower-based HTTP client");

        let http_executor = TokioExecutor::new();
        
        let mut http_connector = hyper_util::client::legacy::connect::HttpConnector::new();
        http_connector.set_nodelay(true);
        http_connector.set_keepalive(Some(Duration::from_secs(60)));
        http_connector.set_connect_timeout(Some(Duration::from_secs(5)));
        http_connector.set_happy_eyeballs_timeout(Some(Duration::from_millis(50)));
        
        let https_connector = HttpsConnectorBuilder::new()
            .with_provider_and_webpki_roots(rustls::crypto::aws_lc_rs::default_provider())
            .expect("Failed to load root certificates")
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .wrap_connector(http_connector);

        let base_client = hyper_util::client::legacy::Client::builder(http_executor)
            .pool_idle_timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(100)
            .build(https_connector);

        let tower_client = ServiceBuilder::new()
            .concurrency_limit(100)
            .timeout(Duration::from_secs(15))
            .service(base_client);

        info!("Optimized Tower HTTP client created");

        Self {
            client: Arc::new(tower_client),
            url,
        }
    }

    #[instrument(skip(self, request))]
    pub async fn execute_request(&self, request: &[u8]) -> Result<Vec<u8>, RpcError> {
        let body = Full::new(Bytes::copy_from_slice(request));
        
        let http_request = Request::builder()
            .method("POST")
            .uri(self.url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "palantiri-tower/1.0")
            .body(body)
            .map_err(|e| RpcError::Transport(format!("Failed to build request: {}", e)))?;

        let mut client = self.client.as_ref().clone();
        let response = client
            .ready()
            .await
            .map_err(|e| RpcError::Transport(format!("Service not ready: {}", e)))?
            .call(http_request)
            .await
            .map_err(|e| RpcError::Transport(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(RpcError::Transport(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let body = response.into_body();
        let body_bytes = body
            .collect()
            .await
            .map_err(|e| RpcError::Transport(format!("Failed to read response: {}", e)))?
            .to_bytes();

        Ok(body_bytes.to_vec())
    }

    pub async fn execute_batch(&self, requests: Vec<Vec<u8>>) -> Result<Vec<Result<Vec<u8>, RpcError>>, RpcError> {
        if requests.is_empty() {
            return Ok(Vec::new());
        }

        if requests.len() == 1 {
            let result = self.execute_request(&requests[0]).await;
            return Ok(vec![result]);
        }

        let futures = requests.into_iter().map(|req| {
            let client = self.client.clone();
            let url = self.url;
            
            async move {
                let body = Full::new(Bytes::from(req));
                
                let http_request = Request::builder()
                    .method("POST")
                    .uri(url)
                    .header("Content-Type", "application/json")
                    .header("User-Agent", "palantiri-tower/1.0")
                    .body(body)
                    .map_err(|e| RpcError::Transport(format!("Failed to build request: {}", e)))?;

                let mut client_clone = client.as_ref().clone();
                let response = client_clone
                    .ready()
                    .await
                    .map_err(|e| RpcError::Transport(format!("Service not ready: {}", e)))?
                    .call(http_request)
                    .await
                    .map_err(|e| RpcError::Transport(format!("Request failed: {}", e)))?;

                if !response.status().is_success() {
                    return Err(RpcError::Transport(format!(
                        "HTTP error: {}",
                        response.status()
                    )));
                }

                let body = response.into_body();
                let body_bytes = body
                    .collect()
                    .await
                    .map_err(|e| RpcError::Transport(format!("Failed to read response: {}", e)))?
                    .to_bytes();

                Ok(body_bytes.to_vec())
            }
        });

        let results = futures::future::join_all(futures).await;
        Ok(results)
    }
}

#[async_trait]
impl Transport for TowerTransport {
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