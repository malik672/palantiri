use std::sync::Arc;

use alloy_primitives::U256;
use async_trait::async_trait;
use palantiri::RpcError;
use parser::{hex_to_u256, parser_for_small_response::Generic};
use serde::Serialize;
use serde_json::json;

#[async_trait]
pub trait Transport: Send + Sync + std::fmt::Debug {
    async fn execute_raw(&self, request: String) -> Result<Vec<u8>, RpcError>;
    async fn execute(&self, request: String) -> Result<String, RpcError>;
}

//: Real time data no need for cache
#[derive(Debug, Clone)]
pub struct RpcClient {
    pub transport: Arc<dyn Transport>,
}

/// Represents an RPC request to a Ethereum node
#[derive(Debug, Serialize)]
pub struct RpcRequest {
    pub jsonrpc: &'static str,
    pub method: &'static str,
    pub params: serde_json::Value,
    pub id: u64,
}

impl RpcClient {
    pub async fn get_gas_price(&self) -> Result<U256, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_gasPrice",
            params: json!([]),
            id: 1,
        };

        let response = self.execute_raw(request).await?;

        match Generic::parse(&response) {
            Some(generic) => {
                let bytes = &response[generic.result_start.0..generic.result_start.1];
                Ok(hex_to_u256(&bytes[2..]))
            }
            None => Err(RpcError::Response("Failed to parse gas price".into())),
        }
    }

    pub async fn execute_raw(&self, request: RpcRequest) -> Result<Vec<u8>, RpcError> {
        let response = self
            .transport
            .execute_raw(serde_json::to_string(&request).expect("convert to string"))
            .await?;

        Ok(response)
    }

    pub async fn execute<T: serde::de::DeserializeOwned>(
        &self,
        request: RpcRequest,
    ) -> Result<T, RpcError> {
        let response = self
            .transport
            .execute(serde_json::to_string(&request).expect("convert to string"))
            .await?;

        serde_json::from_str(&response).map_err(|e| RpcError::Parse(e.to_string()))
    }
}
