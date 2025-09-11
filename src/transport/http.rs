use std::time::Duration;

use async_trait::async_trait;

use crate::{
    hyper_transport::HyperTransport, reqwest_transport::ReqwestTransport, 
    tower_transport::TowerTransport, direct_transport::DirectTransport, 
    direct_reqwest_transport::DirectReqwestTransport, HttpTransport, RpcError,
};

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

    /// Build minimal Alloy-style Hyper transport for maximum performance 
    pub fn build_http_hyper_minimal(self) -> HyperTransport {
        HyperTransport::new_minimal(self.urls[0])
    }

    /// Build ultra-fast Hyper transport to beat Alloy performance
    pub fn build_http_hyper_ultra(self) -> HyperTransport {
        HyperTransport::new_ultra_fast(self.urls[0])
    }

    /// Build realistic Hyper transport for benchmarking (minimal connection pooling like Alloy)
    pub fn build_http_hyper_benchmark(self) -> HyperTransport {
        HyperTransport::new_benchmark_realistic(self.urls[0])
    }

    pub fn build_http_with_config(self, param: HttpTransport) -> HttpTransport {
        HttpTransport::new_with_config(param)
    }

    pub fn build_reqwest(self) -> ReqwestTransport {
        ReqwestTransport::new(self.urls[0])
    }

    pub fn build_reqwest_minimal(self) -> ReqwestTransport {
        ReqwestTransport::new_minimal(self.urls[0])
    }

    pub fn build_reqwest_optimized(self) -> ReqwestTransport {
        ReqwestTransport::new_optimized(self.urls[0])
    }

    pub fn build_reqwest_ultra_fast(self) -> ReqwestTransport {
        ReqwestTransport::new_ultra_fast(self.urls[0])
    }

    pub fn build_tower(self) -> TowerTransport {
        TowerTransport::new(self.urls[0])
    }

    pub fn build_tower_optimized(self) -> TowerTransport {
        TowerTransport::new_optimized(self.urls[0])
    }

    /// Build direct transport for honest benchmarking (fresh connections every time)
    pub fn build_direct(self) -> DirectTransport {
        DirectTransport::new(self.urls[0])
    }

    /// Build direct reqwest transport for honest benchmarking (fresh clients every time)  
    pub fn build_direct_reqwest(self) -> DirectReqwestTransport {
        DirectReqwestTransport::new(self.urls[0])
    }
}
