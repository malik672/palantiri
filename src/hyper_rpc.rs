use alloy::hex;
use alloy::primitives::{Address, BlockNumber, Bytes, FixedBytes, B256, U256, U64};
use async_trait::async_trait;
use parser::types::{FilterParams, TransactionRequest};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::any::Any;
use tokio::sync::Mutex;

use crate::parser::block_parser::parse_block;
use crate::parser::parser_for_small_response::Generic;
use crate::parser::tx_parser::parse_transaction;
use crate::parser::{
    lib::{hex_to_b256, hex_to_u256, hex_to_u64},
    types::{Block, BlockHeader, Log, RawJsonResponse, TransactionTx},
};

use super::*;

#[async_trait]
pub trait Transport: Send + Sync + std::fmt::Debug + Any {
    async fn hyper_execute_raw(&self, request: &'static [u8]) -> Result<Vec<u8>, RpcError>;
    async fn hyper_execute(&self, request: String) -> Result<String, RpcError>;
    async fn hyper_execute_bytes(&self, request: Vec<u8>) -> Result<Vec<u8>, RpcError>;
}

pub enum BlockIdentifier {
    Hash(B256),
    Number(u64),
}
#[derive(Debug)]
struct CacheEntry {
    data: Vec<u8>,
    timestamp: Instant,
}

#[derive(Debug)]
struct BatchingStats {
    optimal_batch_size: usize,
    last_batch_time: Duration,
    samples: Vec<(usize, Duration)>, // (batch_size, duration)
}

#[derive(Debug, Clone)]  
pub struct RpcClient {
    pub transport: Arc<dyn Transport>,
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    batching_stats: Arc<std::sync::Mutex<BatchingStats>>,
}

/// Represents an RPC request to a Ethereum node
#[derive(Debug, Clone, Serialize)]
pub struct RpcRequest {
    pub jsonrpc: &'static str,
    pub method: &'static str,
    pub params: serde_json::Value,
    pub id: u64,
}

impl RpcClient {
    pub fn new<T: Transport + 'static>(transport: T) -> Self {
        Self {
            transport: Arc::new(transport),
            cache: Arc::new(Mutex::new(HashMap::new())),
            batching_stats: Arc::new(std::sync::Mutex::new(BatchingStats {
                optimal_batch_size: 5, // Start with conservative batch size
                last_batch_time: Duration::from_millis(500),
                samples: Vec::new(),
            })),
        }
    }

    fn get_cache_key(&self, request: &RpcRequest) -> String {
        format!("{}:{}", request.method, serde_json::to_string(&request.params).unwrap_or_default())
    }

    fn should_cache(&self, method: &str) -> bool {
        // Cache block requests for recent blocks (they change rarely)
        matches!(method, "eth_getBlockByNumber" | "eth_getBlockByHash" | "eth_getTransactionByHash")
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
            Ok(None)
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

    /// fethces the block by number with smart prefetching
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

        // Trigger predictive prefetching in background
        self.maybe_prefetch_adjacent_blocks(number, full_tx);

        match parse_block(&response_bytes) {
            Some(block) => Ok(Some(block)),
            None => Ok(None),
        }
    }

    fn maybe_prefetch_adjacent_blocks(&self, current_block: u64, full_tx: bool) {
        // Predictively fetch next few blocks in background
        // This is especially useful for scanning applications
        let rpc_clone = self.clone();
        tokio::spawn(async move {
            for offset in 1..=3 {
                if current_block >= offset {
                    let prev_block = current_block - offset;
                    let request = RpcRequest {
                        jsonrpc: "2.0",
                        method: "eth_getBlockByNumber",
                        params: json!([format!("0x{:x}", prev_block), full_tx]),
                        id: 1,
                    };
                    
                    // Only prefetch if not already cached
                    let cache_key = rpc_clone.get_cache_key(&request);
                    let should_fetch = {
                        let cache_result = rpc_clone.cache.try_lock();
                        match cache_result {
                            Ok(cache) => !cache.contains_key(&cache_key),
                            Err(_) => false,
                        }
                    };
                    
                    if should_fetch {
                        let _ = rpc_clone.execute_raw(request).await;
                    }
                }
                
                // Also prefetch next blocks for forward scanning
                let next_block = current_block + offset;
                let request = RpcRequest {
                    jsonrpc: "2.0",
                    method: "eth_getBlockByNumber", 
                    params: json!([format!("0x{:x}", next_block), full_tx]),
                    id: 1,
                };
                
                let cache_key = rpc_clone.get_cache_key(&request);
                let should_fetch = {
                    let cache_result = rpc_clone.cache.try_lock();
                    match cache_result {
                        Ok(cache) => !cache.contains_key(&cache_key),
                        Err(_) => false,
                    }
                };
                
                if should_fetch {
                    let _ = rpc_clone.execute_raw(request).await;
                }
            }
        });
    }

    /// Fetch multiple blocks in a single batch request - much faster for recent blocks
    pub async fn get_blocks_by_numbers(
        &self,
        numbers: Vec<u64>,
        full_tx: bool,
    ) -> Result<Vec<Option<Block>>, RpcError> {
        if numbers.is_empty() {
            return Ok(Vec::new());
        }

        let requests: Vec<RpcRequest> = numbers
            .iter()
            .enumerate()
            .map(|(i, &number)| RpcRequest {
                jsonrpc: "2.0",
                method: "eth_getBlockByNumber",
                params: json!([format!("0x{:x}", number), full_tx]),
                id: i as u64 + 1,
            })
            .collect();

        let response_bytes = self.execute_batch_raw(requests).await?;
        
        // Parse batch response - it should be a JSON array
        let batch_results: Vec<serde_json::Value> = serde_json::from_slice(&response_bytes)
            .map_err(|e| RpcError::Parse(format!("Failed to parse batch response: {}", e)))?;

        let mut blocks = Vec::with_capacity(numbers.len());
        
        for result in batch_results {
            if let Some(result_data) = result.get("result") {
                if result_data.is_null() {
                    blocks.push(None);
                } else {
                    // Convert back to bytes for our zero-copy parser
                    let result_bytes = serde_json::to_vec(result_data)
                        .map_err(|e| RpcError::Parse(e.to_string()))?;
                    
                    match parse_block(&result_bytes) {
                        Some(block) => blocks.push(Some(block)),
                        None => blocks.push(None),
                    }
                }
            } else {
                blocks.push(None);
            }
        }

        Ok(blocks)
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

    pub async fn get_balance(&self, address: Address, state: &str) -> Result<U256, RpcError> {
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

    /// Estimates the gas required to execute a transaction
    ///
    /// This function sends an eth_estimateGas request to an Ethereum node to calculate
    /// the amount of gas needed to execute the given transaction.
    ///
    /// # Arguments
    /// * `tx` - The transaction request details
    /// * `block` - Optional block number to simulate the transaction against
    ///
    /// # Returns
    /// * `Result<U256, RpcError>` - The estimated gas as a U256 value, 0(default for error)
    pub async fn estimate_gas(
        &self,
        tx: &TransactionRequest,
        block: Option<BlockNumber>,
    ) -> Result<U256, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_estimateGas",
            params: json!([tx, block.map(|b| format!("0x{:x}", b))]),
            id: 1,
        };

        let response = self.execute_raw(request).await?;
        match Generic::parse(&response) {
            Some(generic) => {
                let bytes = &response[generic.result_start.0..generic.result_start.1];
                Ok(hex_to_u256(&bytes[2..]))
            }
            None => Ok(U256::ZERO),
        }
    }

    pub async fn new_filter(&self, filter: &FilterParams) -> Result<U256, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_newFilter",
            params: json!([filter]),
            id: 1,
        };

        let response = self.execute_raw(request).await?;
        match Generic::parse(&response) {
            Some(generic) => {
                let bytes = &response[generic.result_start.0..generic.result_start.1];
                Ok(hex_to_u256(&bytes[2..]))
            }
            None => Err(RpcError::Response("Failed to create filter".into())),
        }
    }

    pub async fn new_block_filter(&self) -> Result<U256, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_newBlockFilter",
            params: json!([]),
            id: 1,
        };

        let response = self.execute_raw(request).await?;
        match Generic::parse(&response) {
            Some(generic) => {
                let bytes = &response[generic.result_start.0..generic.result_start.1];
                Ok(hex_to_u256(&bytes[2..]))
            }
            None => Err(RpcError::Response("Failed to create block filter".into())),
        }
    }

    pub async fn get_filter_logs(&self, filter_id: U256) -> Result<Vec<Log>, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getFilterLogs",
            params: json!([format!("0x{:x}", filter_id)]),
            id: 1,
        };

        let response = self.execute_raw(request).await?;
        if let Some(raw_response) = RawJsonResponse::parse(&response) {
            let mut logs = Vec::new();
            for raw_log in raw_response.logs() {
                logs.push(raw_log.to_log());
            }
            Ok(logs)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn syncing(&self) -> Result<bool, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_syncing",
            params: json!([]),
            id: 1,
        };

        let response = self.execute_raw(request).await?;
        match Generic::parse(&response) {
            Some(generic) => {
                let bytes = &response[generic.result_start.0..generic.result_start.1];
                Ok(bytes != b"false")
            }
            None => Err(RpcError::Response("Failed to get sync status".into())),
        }
    }

    ///ISSUE: MAKE THISS USE EXECUTE_RAW
    pub async fn fee_history(
        &self,
        block_count: U64,
        newest_block: U64,
        reward_percentiles: &[f64],
    ) -> Result<Value, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_feeHistory",
            params: json!([
                format!("0x{:x}", block_count),
                format!("0x{:x}", newest_block),
                reward_percentiles
            ]),
            id: 1,
        };

        self.execute(request).await
    }

    ///ISSUE: MAKE THISS USE EXECUTE_RAW
    pub async fn get_proof(
        &self,
        address: Address,
        storage_keys: &[B256],
        block: BlockNumber,
    ) -> Result<Value, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getProof",
            params: json!([
                format!("0x{:x}", address),
                storage_keys,
                format!("0x{:x}", block)
            ]),
            id: 1,
        };

        self.execute(request).await
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

    // pub async fn get_eth_call()

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
        let cache_key = self.get_cache_key(&request);
        
        // Check cache first if this request type should be cached
        if self.should_cache(request.method) {
            if let Ok(mut cache) = self.cache.try_lock() {
                if let Some(entry) = cache.get(&cache_key) {
                    // Cache entries expire after 30 seconds for recent blocks
                    if entry.timestamp.elapsed() < Duration::from_secs(30) {
                        return Ok(entry.data.clone());
                    } else {
                        cache.remove(&cache_key);
                    }
                }
            }
        }

        self.execute_raw_internal(request).await
    }

    async fn execute_raw_internal(&self, request: RpcRequest) -> Result<Vec<u8>, RpcError> {
        // Pre-allocate with larger capacity based on typical request sizes
        let mut buffer = Vec::with_capacity(512);
        
        // Build JSON more efficiently
        buffer.extend_from_slice(b"{\"jsonrpc\":\"");
        buffer.extend_from_slice(request.jsonrpc.as_bytes());
        buffer.extend_from_slice(b"\",\"method\":\"");
        buffer.extend_from_slice(request.method.as_bytes());
        buffer.extend_from_slice(b"\",\"params\":");
        
        // Serialize params directly into buffer - this is already optimal
        serde_json::to_writer(&mut buffer, &request.params)
            .map_err(|e| RpcError::Parse(e.to_string()))?;
            
        buffer.extend_from_slice(b",\"id\":");
        
        // Optimize integer writing
        let id_str = request.id.to_string();
        buffer.extend_from_slice(id_str.as_bytes());
        buffer.push(b'}');

        // Use a safer approach - create new transport method that takes Vec<u8>
        let response = self.transport.hyper_execute_bytes(buffer).await?;

        // Cache the response if appropriate
        if self.should_cache(request.method) {
            let cache_key = self.get_cache_key(&request);
            
            if let Ok(mut cache) = self.cache.try_lock() {
                cache.insert(cache_key, CacheEntry {
                    data: response.clone(),
                    timestamp: Instant::now(),
                });
                
                // Clean old entries occasionally (keep cache size manageable)
                if cache.len() > 1000 {
                    let now = Instant::now();
                    cache.retain(|_, entry| now.duration_since(entry.timestamp) < Duration::from_secs(300));
                }
            }
        }

        Ok(response)
    }

    /// Execute multiple RPC requests in a single batch call
    pub async fn execute_batch_raw(&self, requests: Vec<RpcRequest>) -> Result<Vec<u8>, RpcError> {
        if requests.is_empty() {
            return Err(RpcError::Parse("Empty batch request".into()));
        }
        
        if requests.len() == 1 {
            return self.execute_raw(requests.into_iter().next().unwrap()).await;
        }

        let start_time = Instant::now();
        let batch_size = requests.len();

        // Determine if we should use concurrent multiplexed requests or single batch
        let use_multiplexing = {
            let stats = self.batching_stats.lock().unwrap();
            // Use multiplexing for smaller batches where connection overhead is amortized
            batch_size <= 10 && stats.optimal_batch_size > batch_size * 2
        };

        if use_multiplexing {
            // Use connection multiplexing for concurrent individual requests
            let individual_requests: Vec<Vec<u8>> = requests
                .iter()
                .map(|req| {
                    format!(
                        r#"{{"jsonrpc":"{}","method":"{}","params":{},"id":{}}}"#,
                        req.jsonrpc, req.method, req.params, req.id
                    ).into_bytes()
                })
                .collect();

            // Try to cast transport to HyperTransport to use batch multiplexing
            if let Some(hyper_transport) = (&*self.transport as &dyn std::any::Any)
                .downcast_ref::<crate::hyper_transport::HyperTransport>() 
            {
                let results = hyper_transport.hyper_execute_bytes_batch(individual_requests).await
                    .map_err(|e| RpcError::Transport(format!("Multiplexed batch failed: {}", e)))?;
                
                // Combine successful results into a JSON array
                let mut response_array = Vec::new();
                response_array.push(b'[');
                
                let mut first = true;
                for result in results {
                    match result {
                        Ok(response_bytes) => {
                            if !first {
                                response_array.push(b',');
                            }
                            response_array.extend_from_slice(&response_bytes);
                            first = false;
                        }
                        Err(_) => {
                            // Skip failed requests in multiplexed mode
                            continue;
                        }
                    }
                }
                
                response_array.push(b']');
                
                // Update batching stats with multiplexing performance
                let elapsed = start_time.elapsed();
                {
                    let mut stats = self.batching_stats.lock().unwrap();
                    stats.samples.push((batch_size, elapsed));
                    if stats.samples.len() > 20 {
                        stats.samples.remove(0);
                    }
                }
                
                return Ok(response_array);
            }
        }

        // Fallback to standard JSON-RPC batch request
        let mut buffer = Vec::with_capacity(requests.len() * 512);
        buffer.push(b'[');
        
        for (i, request) in requests.iter().enumerate() {
            if i > 0 {
                buffer.push(b',');
            }
            
            buffer.extend_from_slice(b"{\"jsonrpc\":\"");
            buffer.extend_from_slice(request.jsonrpc.as_bytes());
            buffer.extend_from_slice(b"\",\"method\":\"");
            buffer.extend_from_slice(request.method.as_bytes());
            buffer.extend_from_slice(b"\",\"params\":");
            
            serde_json::to_writer(&mut buffer, &request.params)
                .map_err(|e| RpcError::Parse(e.to_string()))?;
                
            buffer.extend_from_slice(b",\"id\":");
            let id_str = request.id.to_string();
            buffer.extend_from_slice(id_str.as_bytes());
            buffer.push(b'}');
        }
        
        buffer.push(b']');
        
        let response = self.transport.hyper_execute_bytes(buffer).await?;
        
        // Update batching performance stats
        let elapsed = start_time.elapsed();
        {
            let mut stats = self.batching_stats.lock().unwrap();
            stats.last_batch_time = elapsed;
            stats.samples.push((batch_size, elapsed));
            if stats.samples.len() > 20 {
                stats.samples.remove(0);
            }
            
            // Recalculate optimal batch size based on recent samples
            if stats.samples.len() >= 5 {
                let avg_time_per_request: f64 = stats.samples.iter()
                    .map(|(size, duration)| duration.as_millis() as f64 / *size as f64)
                    .sum::<f64>() / stats.samples.len() as f64;
                
                // Optimal batch size balances latency vs throughput
                stats.optimal_batch_size = ((500.0 / avg_time_per_request) as usize).clamp(1, 20);
            }
        }
        
        Ok(response)
    }

    pub async fn execute<T: DeserializeOwned>(&self, request: RpcRequest) -> Result<T, RpcError> {
        let request_str = format!(
            r#"{{"jsonrpc":"{}","method":"{}","params":{},"id":{}}}"#,
            request.jsonrpc, request.method, request.params, request.id
        );
        let response = self.transport.hyper_execute(request_str).await?;

        serde_json::from_str(&response).map_err(|e| RpcError::Parse(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::address;

    use super::*;
    use crate::transport::http::TransportBuilder;
    use std::time::Instant;

    #[tokio::test]
    async fn test_request() {
        let time = Instant::now();

        let client = RpcClient::new(
            TransportBuilder::new("https://mainnet.infura.io/v3/2DCsBRUv8lDFmznC1BGik1pFKAL")
                .build_http_hyper(),
        );

        let tx: TransactionRequest = TransactionRequest {
            from: Some(address!("8f54C8c2df62c94772ac14CcFc85603742976312")),
            to: Some(address!("44aa93095d6749a706051658b970b941c72c1d53")),
            gas: None,
            gas_price: Some(U256::from(26112348709 as u64)),
            value: None,
            data: Some("0xdd9c5f960000000000000000000000000d500b1d8e8ef31e21c99d1db9a6444d3adf12700000000000000000000000000000000000000000000000056bc75e2d631000000000000000000000000000000b3f868e0be5597d5db7feb59e1cadbb0fdda50a000000000000000000000000000000000000000000000001e1291b1bf0494000000000000000000000000000000000000000000000000001de460b131125fe970000000000000000000000008f54c8c2df62c94772ac14ccfc856037429763120000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000e0020d500B1d8E8eF31E21C99d1Db9A6444d3ADf12700215550133C4F0043E2e988b3c2e9C77e2C670eFe709Bfe30185CD07Ea01423b1E937929B44E4Ad8c40BbB5E7100ffff0186f1d8390222A3691C28938eC7404A1661E618e00185CD07Ea01423b1E937929B44E4Ad8c40BbB5E7100017ceB23fD6bC0adD59E62ac25578270cFf1b9f619026aaa010312692E9cADD3dDaaCE2E112A4e36397bd2f18a0085CD07Ea01423b1E937929B44E4Ad8c40BbB5E7100ffff01Ff5713FdbAD797b81539b5F9766859d4E050a6CC0085CD07Ea01423b1E937929B44E4Ad8c40BbB5E7100".to_string()),
            nonce: None,
        };

        let x = client.estimate_gas(&tx, None).await.unwrap();
        println!("{:?}{:?}", time.elapsed(), x);
    }

    #[tokio::test]
    async fn test_get_block() {
        let rpc = RpcClient::new(
            TransportBuilder::new("https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4")
                .build_http_hyper(),
        );

        let x = rpc.get_block_by_number(22349461, true);
        println!("{:?}", x.await.unwrap());
    }
}
