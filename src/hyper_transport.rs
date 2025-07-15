use std::time::Duration;
use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::header::HeaderValue;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::rt::TokioExecutor;
use tracing::{debug, info};

use crate::hyper_rpc::Transport;
use crate::RpcError;

const DURATION_60: Duration = std::time::Duration::from_secs(30);
const CONTENT_TYPE_JSON: HeaderValue = HeaderValue::from_static("application/json");
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);



#[derive(Debug, Clone)]
pub struct HyperTransport {
 client: hyper_util::client::legacy::Client<
        HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
        http_body_util::Full<::hyper::body::Bytes>,
    >,
    url: String,
}

impl HyperTransport {
    pub fn new(url: String) -> Self {
        debug!("Creating new HyperTransport for URL: {}", url);

        let http_executor = TokioExecutor::new();

        let mut http_connector = hyper_util::client::legacy::connect::HttpConnector::new();

        http_connector.set_connect_timeout(Some(CONNECTION_TIMEOUT));

        http_connector.set_keepalive(Some(Duration::from_secs(60)));
        http_connector.set_send_buffer_size(Some(1024 * 1024 ));


        http_connector.enforce_http(false);
    
        let https_connector = HttpsConnectorBuilder::new()
              .with_provider_and_webpki_roots(rustls::crypto::aws_lc_rs::default_provider())
            .expect("Failed to load native root certificates")
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .wrap_connector(http_connector);

        let client = hyper_util::client::legacy::Client::builder(http_executor)
            .pool_idle_timeout(DURATION_60)
            .pool_max_idle_per_host(50)
            .retry_canceled_requests(false)
            .build(https_connector);

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

        Ok(body_bytes.into())
    }
}
