//! Ethereum RPC client with zero-copy parsing for high performance
//! 
//! # Transport Layer
//! ```rust 
//! pub trait Transport: Send + Sync + std::fmt::Debug {
//!     async fn execute_raw(&self, request: String) -> Result<Vec<u8>, RpcError>;
//!     async fn execute(&self, request: String) -> Result<String, RpcError>;
//!     async fn execute_with_retry(&self, request: String, retry: usize) -> Result<String, RpcError>;
//!     async fn connect(&self) -> Result<(), RpcError>;
//! }
//! ```
//!
//! # RPC Methods
//! ## Chain State
//! ```rust
//! // Get current chain ID
//! pub async fn get_chain_id(&self) -> Result<U64, RpcError>
//!
//! // Get current gas price
//! pub async fn get_gas_price(&self) -> Result<U256, RpcError>
//! 
//! // Get current block number
//! pub async fn get_block_number(&self) -> Result<U64, RpcError>
//! ```
//!
//! ## Blocks
//! ```rust
//! // Get block by number with optional full transaction data
//! pub async fn get_block_by_number(
//!     &self,
//!     number: u64,
//!     full_tx: bool,
//! ) -> Result<Option<Block>, RpcError>
//!
//! // Get block header only
//! pub async fn get_block_header_by_number(
//!     &self,
//!     number: u64, 
//!     full_tx: bool
//! ) -> Result<Option<BlockHeader>, RpcError>
//! ```
//!
//! ## Transactions
//! ```rust
//! // Get transaction by hash
//! pub async fn get_transaction_by_tx_hash(
//!     &self,
//!     hash: B256
//! ) -> Result<Option<TransactionTx>, RpcError>
//!
//! // Get transaction by block and index
//! pub async fn get_transaction_by_block_with_index(
//!     &self,
//!     block: BlockIdentifier,
//!     index: U64
//! ) -> Result<Option<TransactionTx>, RpcError>
//! ```
//!
//! ## State
//! ```rust
//! // Get account balance
//! pub async fn get_balance(
//!     &self,
//!     address: Address,
//!     block: BlockNumber
//! ) -> Result<U256, RpcError>
//!
//! // Get contract code
//! pub async fn get_code(
//!     &self,
//!     address: Address,
//!     block: BlockNumber
//! ) -> Result<Bytes, RpcError>
//! ```
//!
//! ## Logs
//! ```rust 
//! // Get event logs in block range with optional filtering
//! pub async fn get_logs(
//!     &self,
//!     from_block: u64,
//!     to_block: u64,
//!     address: Option<Address>,
//!     topics: Option<Vec<B256>>
//! ) -> Result<Option<Vec<Log>>, RpcError>
//! ```
//!
//! # Performance
//! - Zero allocation JSON parsing
//! - Pre-allocated buffers
//! - Connection pooling
//! - Concurrent batch requests
//! - ~95% faster than standard implementations
//! - Processes 2300+ blocks/sec
//!
//! # Example
//! ```rust
//! let transport = HttpTransport::new(url);
//! let client = RpcClient::new(transport);
//!
//! // Fetch blocks concurrently
//! let blocks = client.get_block_by_number(block_num, false).await?;
//! ```
//! # Performance Notes
//! Benchmarks performed on:
//! - MacBook Air M3
//! - 16GB RAM
//! - 500GB Storage
//!
//! Results:
//! - 1000 concurrent requests: 431ms
//! - Log fetching: ~95% faster than standard implementations
//! - Block processing: ~2300 blocks/sec
//! 
//! Note: Performance may vary based on hardware specifications