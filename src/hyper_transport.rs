use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use futures::task;
use http_body_util::BodyExt;
use hyper::header::HeaderValue;
use hyper_tls::HttpsConnector;
use tower::limit::rate::Rate;
use tower::{Service, ServiceBuilder, ServiceExt};
use tracing::{debug, debug_span, error, info, trace, Instrument};
use hyper_util::client::legacy::{Builder, Client};
use hyper_util::rt::TokioExecutor;

use crate::hyper_rpc::Transport;
use crate::RpcError;

const DURATION_60: Duration = std::time::Duration::from_secs(60);
const CONTENT_TYPE_JSON: HeaderValue = HeaderValue::from_static("application/json");

#[derive(Debug, Clone)]
pub struct HyperTransport {
    client: hyper_util::client::legacy::Client<
        hyper_tls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
        http_body_util::Full<::hyper::body::Bytes>,
    >,

    url: String,
}

/// Implementation of Tower's Service trait for Hyper_Execute
impl Service<String> for HyperTransport {
    type Response = String;
    type Error = RpcError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: String) -> Self::Future {
        let this = self.clone();
        let span = debug_span!("HyperTransport::call");
        Box::pin(async move {
            this.hyper_execute(request).instrument(span).await
        })
    }

}

impl Service<Vec<u8>> for HyperTransport {
    type Response = Vec<u8>;
    type Error = RpcError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Vec<u8>) -> Self::Future {
        let this = self.clone();
        let span = debug_span!("HyperTransport::call");
        Box::pin(async move {
            this.hyper_execute_raw(request).instrument(span).await
        })
    }

}


impl HyperTransport {
    pub fn new(url: String) -> Self {
        debug!("Creating new HyperTransport for URL: {}", url);
           println!("Creating new RpcClient with transport: {:?}", 10);

        let http_executor = TokioExecutor::new();

        let mut http_connector = hyper_util::client::legacy::connect::HttpConnector::new();
        // http_connector.set_nodelay(true); 
        http_connector.enforce_http(false);
        // http_connector.set_reuse_address(true);
        // http_connector.set_keepalive(Some(std::time::Duration::from_secs(30))); 
    
        let client = hyper_util::client::legacy::Client::builder(http_executor)
        .pool_idle_timeout(DURATION_60)
        .pool_max_idle_per_host(32)
        .retry_canceled_requests(true)
            .build(hyper_tls::HttpsConnector::new_with_connector(http_connector));

        info!("HyperTransport client created successfully");
        Self { client, url }
    }
}

#[async_trait]
impl Transport for HyperTransport {
    async fn hyper_execute(&self, request: String) -> Result<String, RpcError> {
        let url = &self.url;

        let reqs = request.as_bytes().to_owned().into();

        let req = hyper::Request::builder()
            .method(hyper::Method::POST)
            .uri(url.as_str())
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

    async fn hyper_execute_raw(&self, request: Vec<u8>) -> Result<Vec<u8>, RpcError> {
        let url = &self.url;

        let reqs = Bytes::from(request);

        let req = hyper::Request::builder()
            .method(hyper::Method::POST)
            .uri(url.as_str())
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

        Ok(body_bytes.to_vec())
    }
}
