use alloy::hex;
use alloy::primitives::{keccak256, Address, BlockNumber, Bytes, FixedBytes, B256, U256, U64};
use async_trait::async_trait;
use lru::LruCache;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::error::Error;
use std::num::NonZeroUsize;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use std::{fmt::Display, net::SocketAddr, sync::Arc};

use crate::types::{Block, BlockHeader};

use super::*;

#[async_trait]
pub trait Transport: Send + Sync + std::fmt::Debug {
    async fn execute(&self, request: String) -> Result<String, RpcError>;
    async fn execute_with_retry(&self, request: String, retry: usize) -> Result<String, RpcError>;
    async fn connect(&self) -> Result<(), RpcError>;
}

#[async_trait]
pub trait Method {
    type Params: Serialize;
    type Response: DeserializeOwned;

    fn name() -> &'static str;
    fn params(&self) -> Self::Params;
}

#[derive(Debug, Clone)]
struct CacheEntry {
    response: String,
    timestamp: Instant,
}

#[derive(Debug)]
pub struct RequestCache {
    cache: LruCache<B256, CacheEntry>,
    ttl: Duration,
}

#[derive(Debug, Clone)]
pub struct RpcClient {
    pub transport: Arc<dyn Transport>,
    pub cache: Arc<RwLock<RequestCache>>,
}

/// Represents an RPC request to a Ethereum node
#[derive(Debug, Serialize)]
pub struct RpcRequest {
    pub jsonrpc: &'static str,
    pub method: &'static str,
    pub params: serde_json::Value,
    pub id: u64,
}

impl RequestCache {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
            ttl,
        }
    }

    pub fn get(&mut self, key: &B256) -> Option<String> {
        if let Some(entry) = self.cache.get(key) {
            if entry.timestamp.elapsed() < self.ttl {
                Some(entry.response.to_string())
            } else {
                self.cache.pop(key);
                None
            }
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: B256, value: String) {
        self.cache.put(
            key,
            CacheEntry {
                response: value,
                timestamp: Instant::now(),
            },
        );
    }
}

impl Default for RequestCache {
    fn default() -> Self {
        Self::new(100, Duration::from_secs(60))
    }
}

impl RpcClient {
    pub fn new<T: Transport + 'static>(transport: T) -> Self {
        Self {
            transport: Arc::new(transport),
            cache: Arc::new(RwLock::new(RequestCache::default())),
        }
    }

    pub async fn get_chain_id(&self) -> Result<Value, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_chainId",
            params: json!([]),
            id: 1,
        };

        self.execute_with_cache(request).await
    }

    pub async fn get_gas_price(&self) -> Result<Value, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_gasPrice",
            params: json!([]),
            id: 1,
        };

        self.execute_with_cache(request).await
    }

    pub async fn get_max_priority_fee_per_gas(&self) -> Result<Value, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_maxPriorityFeePerGas",
            params: json!([]),
            id: 1,
        };

        self.execute_with_cache(request).await
    }

    pub async fn get_block_number(&self) -> Result<u64, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_blockNumber",
            params: json!([]),
            id: 1,
        };

        // Send the RPC request
        let response: Value = self.execute(request).await?;

        // Extract result
        let hex_str = response["result"]
            .as_str()
            .ok_or_else(|| RpcError::Response("Missing result field".to_string()))?;

        // Convert the hexadecimal string to a decimal number
        let block_number = u64::from_str_radix(hex_str.trim_start_matches("0x"), 16)
            .map_err(|e| RpcError::Response(format!("Failed to parse block number: {}", e)))?;

        Ok(block_number)
    }

    pub async fn get_block_by_number(&self, number: u64, full_tx: bool) -> Result<BlockHeader, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getBlockByNumber",
            params: json!([format!("0x{:x}", number), full_tx]),
            id: 1,
        };

        let response: Value = self.execute_with_cache(request).await?;
        let block: BlockHeader =
            serde_json::from_value(response["result"].clone()).map_err(|e| RpcError::Response(e.to_string()))?;

        Ok(block)
    }

    pub async fn get_block_by_hash(
        &self,
        hash: FixedBytes<32>,
        full_tx: bool,
    ) -> Result<Block, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getBlockByNumber",
            params: json!([format!("0x{:x}", hash), full_tx]),
            id: 1,
        };

        let response: Value = self.execute_with_cache(request).await?;
        Ok((Block::default()))
    }

    pub async fn get_balance(
        &self,
        address: Address,
        block: BlockNumber,
    ) -> Result<U256, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getBalance",
            params: json!([format!("0x{:x}", address), block]),
            id: 1,
        };

        self.execute_with_cache(request).await
    }

    pub async fn get_code(&self, address: Address, block: BlockNumber) -> Result<Bytes, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getCode",
            params: json!([format!("0x{:x}", address), block]),
            id: 1,
        };

        self.execute_with_cache(request).await
    }

    pub async fn get_storage_at(
        &self,
        address: Address,
        slot: U256,
        block: BlockNumber,
    ) -> Result<B256, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getStorageAt",
            params: json!([format!("0x{:x}", address), format!("0x{:x}", slot), block]),
            id: 1,
        };

        self.execute_with_cache(request).await
    }

    pub async fn get_transaction_count(
        &self,
        address: Address,
        block: BlockNumber,
    ) -> Result<U64, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getTransactionCount",
            params: json!([format!("0x{:x}", address), block]),
            id: 1,
        };

        self.execute_with_cache(request).await
    }

    pub async fn send_raw_transaction(&self, data: Bytes) -> Result<B256, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_sendRawTransaction",
            params: json!([format!("0x{}", hex::encode(&data))]),
            id: 1,
        };

        self.execute_with_cache(request).await
    }

    pub async fn get_transaction_receipt(&self, hash: FixedBytes<32>) -> Result<Value, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getTransactionReceipt",
            params: json!([format!("0x{:x}", hash)]),
            id: 1,
        };

        self.execute_with_cache(request).await
    }

    pub async fn get_block_receipts(&self, block: BlockNumber) -> Result<Value, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getBlockReceipts",
            params: json!([format!("0x{:x}", block)]),
            id: 1,
        };
        self.execute_with_cache(request).await
    }

    pub async fn execute_with_cache<T: DeserializeOwned>(
        &self,
        request: RpcRequest,
    ) -> Result<T, RpcError> {
        let key = keccak256(
            serde_json::to_string(&request)
                .expect("can't convert to string ")
                .as_bytes(),
        );

        // Try read lock first for cache access
        if let Ok(mut cache) = self.cache.write() {
            if let Some(cached) = cache.get(&key) {
                return serde_json::from_str(&cached).map_err(|e| RpcError::Parse(e.to_string()));
            }
        }

        // Execute request if cache miss
        let response = self
            .transport
            .execute(serde_json::to_string(&request).expect("convert to string"))
            .await?;

        // Update cache with write lock
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(key, response.clone());
        }

        serde_json::from_str(&response).map_err(|e| RpcError::Parse(e.to_string()))
    }

    pub async fn execute<T: DeserializeOwned>(
        &self,
        request: RpcRequest,
    ) -> Result<T, RpcError> {

        // Execute request if cache miss
        let response = self
            .transport
            .execute(serde_json::to_string(&request).expect("convert to string"))
            .await?;


        serde_json::from_str(&response).map_err(|e| RpcError::Parse(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_cache() {
        let mut cache = RequestCache::default();
        let key = B256::ZERO;
        let value = "test".to_string();

        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key), Some(value));
    }
}
