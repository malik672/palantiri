use alloy::hex;
use alloy::primitives::{Address, BlockNumber, Bytes, FixedBytes, B256, U256, U64};
use async_trait::async_trait;
use lru::LruCache;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::num::NonZeroUsize;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use parser::block_parser::parse_block;
use parser::parser_for_small_response::Generic;
use parser::tx_parser::parse_transaction;
use parser::{{hex_to_b256, hex_to_u256, hex_to_u64}, types::{Block, BlockHeader, Log, RawJsonResponse, TransactionTx}};


use super::*;

#[async_trait]
pub trait Transport: Send + Sync + std::fmt::Debug {
    async fn execute_raw(&self, request: String) -> Result<Vec<u8>, RpcError>;
    async fn execute(&self, request: String) -> Result<String, RpcError>;
    async fn execute_with_retry(&self, request: String, retry: usize) -> Result<String, RpcError>;
    async fn connect(&self) -> Result<(), RpcError>;
}

pub enum BlockIdentifier {
    Hash(B256),
    Number(u64),
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

    pub async fn get_chain_id(&self) -> Result<U64, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_chainId",
            params: json!([]),
            id: 1,
        };

        let response = self.execute_raw(request).await?;

        match Generic::parse(&response) {
            Some(generic) => {
                let bytes = &response[generic.result_start.0..generic.result_start.1];
                Ok(hex_to_u64(&bytes[2..]))
            }
            None => Err(RpcError::Response("Failed to parse chain ID".into())),
        }
    }

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

    pub async fn get_max_priority_fee_per_gas(&self) -> Result<U256, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_maxPriorityFeePerGas",
            params: json!([]),
            id: 1,
        };

        let response = self.execute_raw(request).await?;

        match Generic::parse(&response) {
            Some(generic) => {
                let bytes = &response[generic.result_start.0..generic.result_start.1];
                Ok(hex_to_u256(&bytes[2..]))
            }
            None => Err(RpcError::Response(
                "Failed to parse max priority fee".into(),
            )),
        }
    }

    pub async fn get_block_number(&self) -> Result<U64, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_blockNumber",
            params: json!([]),
            id: 1,
        };

        let response = self.execute_raw(request).await?;

        match Generic::parse(&response) {
            Some(generic) => {
                let bytes = &response[generic.result_start.0..generic.result_start.1];
                Ok(hex_to_u64(&bytes[2..]))
            }
            None => Err(RpcError::Response("Failed to parse block number".into())),
        }
    }

    pub async fn get_logs(
        &self,
        from_block: u64,
        to_block: u64,
        address: Option<Address>,
        topics: Option<Vec<B256>>,
    ) -> Result<Option<Vec<Log>>, RpcError> {
        //pre allocation does nothing here but in terms of complexity might offer substantial perf
        let params = {
            let mut obj = serde_json::Map::with_capacity(4);
            obj.insert("fromBlock".into(), format!("0x{:x}", from_block).into());
            obj.insert("toBlock".into(), format!("0x{:x}", to_block).into());
            if let Some(addr) = address {
                obj.insert("address".into(), format!("0x{:x}", addr).into());
            }
            if let Some(t) = topics {
                obj.insert("topics".into(), json!(t));
            }
            json!([obj])
        };

        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getLogs",
            params,
            id: 1,
        };

        let response: Vec<u8> = self.execute_raw(request).await?;

        if let Some(raw_response) = RawJsonResponse::parse(&response) {
            let log_count = raw_response.data[raw_response.result_start..raw_response.result_end]
                .iter()
                .filter(|&&b| b == b'{')
                .count();

            let mut logs = Vec::with_capacity(log_count);
            for raw_log in raw_response.logs() {
                logs.push(raw_log.to_log());
            }
            Ok(Some(logs))
        } else {
            return Ok(None);
        }
    }

    pub async fn get_transaction_by_tx_hash(
        &self,
        block_hash: B256,
    ) -> Result<Option<TransactionTx>, RpcError> {
        let params = json!([format!("0x{:x}", block_hash),]);

        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getTransactionByHash",
            params,
            id: 1,
        };

        let response_bytes = self.execute_raw(request).await?;

        match parse_transaction(&response_bytes) {
            Some(tx) => Ok(Some(tx)),
            None => Ok(None),
        }
    }

    pub async fn get_transaction_by_block_with_index(
        &self,
        block: BlockIdentifier,
        index: U64,
    ) -> Result<Option<TransactionTx>, RpcError> {
        let block_param = match block {
            BlockIdentifier::Hash(hash) => format!("0x{:x}", hash),
            BlockIdentifier::Number(num) => format!("0x{:x}", num),
        };

        let params = json!([block_param, format!("0x{:x}", index),]);

        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getTransactionByBlockAndIndex",
            params,
            id: 1,
        };

        let response_bytes = self.execute_raw(request).await?;

        match parse_transaction(&response_bytes) {
            Some(tx) => Ok(Some(tx)),
            None => Ok(None),
        }
    }

    /// fethces the block by number
    pub async fn get_block_by_number(
        &self,
        number: u64,
        full_tx: bool,
    ) -> Result<Option<Block>, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getBlockByNumber",
            params: json!([format!("0x{:x}", number), full_tx]),
            id: 1,
        };

        let response_bytes: Vec<u8> = self.execute_raw(request).await?;

        match parse_block(&response_bytes) {
            Some(block) => Ok(Some(block)),
            None => Ok(None),
        }
    }

    ///this just extracts the header of the block
    /// fethces the block by number then extracts the header
    pub async fn get_block_header_by_number(
        &self,
        number: u64,
        full_tx: bool,
    ) -> Result<Option<BlockHeader>, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getBlockByNumber",
            params: json!([format!("0x{:x}", number), full_tx]),
            id: 1,
        };

        let response: Value = self.execute(request).await?;

        if response["result"].is_null() {
            return Ok(None);
        }

        //FROM BENCHMARK CLONING HERE HAS NO EFFECT ON LATENCY(STUPID RIGHT????????)
        let block: BlockHeader = serde_json::from_value(response["result"].clone())
            .map_err(|e| RpcError::Response(e.to_string()))?;

        Ok(Some(block))
    }

    ///this just extracts the header of the block
    /// fethces the block by tag then extracts the header
    /// possibble tags are ["LATEST"], ["EARLIEST"], ["PENDING"],["SAFE"], ["FINALIZED"]
    pub async fn get_block_header_with_tag(
        &self,
        tag: &str,
        full_tx: bool,
    ) -> Result<Option<BlockHeader>, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getBlockByNumber",
            params: json!([tag, full_tx]),
            id: 1,
        };

        let response: Value = self.execute(request).await?;

        if response["result"].is_null() {
            return Ok(None);
        }

        let block: BlockHeader = serde_json::from_value(response["result"].clone())
            .map_err(|e| RpcError::Response(e.to_string()))?;

        Ok(Some(block))
    }

    pub async fn get_block_by_hash(
        &self,
        hash: FixedBytes<32>,
        full_tx: bool,
    ) -> Result<Option<BlockHeader>, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getBlockByHash",
            params: json!([format!("0x{:x}", hash), full_tx]),
            id: 1,
        };

        let response: Value = self.execute(request).await?;

        //Cloning does not affect latency here from benchmark
        let block: BlockHeader = serde_json::from_value(response["result"].clone())
            .map_err(|e| RpcError::Response(e.to_string()))?;
        Ok(Some(block))
    }

    pub async fn get_balance(
        &self,
        address: Address,
        state: &str,
    ) -> Result<U256, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getBalance",
            params: json!([format!("0x{:x}", address), state]),
            id: 1,
        };

        let response = self.execute_raw(request).await?;

        match Generic::parse(&response) {
            Some(generic) => {
                let bytes = &response[generic.result_start.0..generic.result_start.1];
                Ok(hex_to_u256(&bytes[2..]))
            }
            None => Err(RpcError::Response("Failed to parse balance".into())),
        }
    }

    pub async fn get_code(&self, address: Address, block: String) -> Result<Bytes, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getCode",
            params: json!([format!("0x{:x}", address), block]),
            id: 1,
        };

        let response = self.execute_raw(request).await?;

        match Generic::parse(&response) {
            Some(generic) => {
                let bytes = &response[generic.result_start.0..generic.result_start.1];
                Ok(Bytes::from_str(&String::from_utf8_lossy(bytes)).unwrap())
            }
            None => Err(RpcError::Response("Failed to parse code".into())),
        }
    }

    pub async fn get_storage_at(
        &self,
        address: Address,
        slot: B256,
        block: String,
    ) -> Result<B256, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getStorageAt",
            params: json!([format!("0x{:x}", address), format!("0x{:x}", slot), block]),
            id: 1,
        };

        let response = self.execute_raw(request).await?;

        match Generic::parse(&response) {
            Some(generic) => {
                let bytes = &response[generic.result_start.0..generic.result_start.1];
                Ok(hex_to_b256(&bytes[2..]))
            }
            None => Err(RpcError::Response("Failed to parse storage".into())),
        }
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

        let response = self.execute_raw(request).await?;

        match Generic::parse(&response) {
            Some(generic) => {
                let bytes = &response[generic.result_start.0..generic.result_start.1];
                Ok(hex_to_u64(&bytes[2..]))
            }
            None => Err(RpcError::Response(
                "Failed to parse transaction count".into(),
            )),
        }
    }

    pub async fn send_raw_transaction(&self, data: Bytes) -> Result<B256, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_sendRawTransaction",
            params: json!([format!("0x{}", hex::encode(&data))]),
            id: 1,
        };

        self.execute(request).await
    }

    pub async fn get_transaction_receipt(&self, hash: B256) -> Result<Value, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getTransactionReceipt",
            params: json!([format!("0x{:x}", hash)]),
            id: 1,
        };

        self.execute(request).await
    }

    pub async fn get_block_receipts(&self, block: BlockNumber) -> Result<Value, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getBlockReceipts",
            params: json!([format!("0x{:x}", block)]),
            id: 1,
        };
        self.execute(request).await
    }

    pub async fn execute_raw(&self, request: RpcRequest) -> Result<Vec<u8>, RpcError> {
        let response = self
            .transport
            .execute_raw(serde_json::to_string(&request).expect("convert to string"))
            .await?;

        Ok(response)
    }

    pub async fn execute<T: DeserializeOwned>(&self, request: RpcRequest) -> Result<T, RpcError> {
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
