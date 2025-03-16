# Ethereum RPC Client

A high-performance Ethereum RPC client with zero-copy parsing for maximum efficiency.

## Features

-   **Zero Allocation JSON Parsing**: Optimized for speed and efficiency.
-   **Connection Pooling**: Reuse connections for faster requests.
-   **Concurrent Batch Requests**: Handle multiple requests simultaneously.
-   **High Throughput**: Processes 2300+ blocks/second.

----------

## Transport Layer

The transport layer is responsible for executing RPC requests.



```
pub trait Transport: Send + Sync + std::fmt::Debug {  
    async fn execute_raw(&self, request: String) -> Result<Vec<u8>, RpcError>;  
    async fn execute(&self, request: String) -> Result<String, RpcError>;  
    async fn execute_with_retry(&self, request: String, retry: usize) -> Result<String, RpcError>;  
    async fn connect(&self) -> Result<(), RpcError>;  
}
``` 

----------

## RPC Methods

### Chain State



```// Get current chain ID 
pub async fn get_chain_id(&self) -> Result<U64, RpcError>  

// Get current gas price 
pub async fn get_gas_price(&self) -> Result<U256, RpcError>  

// Get current block number 
pub async fn get_block_number(&self) -> Result<U64, RpcError>
```

### Blocks


```// Get block by number with optional full transaction data 
pub async fn get_block_by_number(  
    &self,  
    number: u64,  
    full_tx: bool,  
) -> Result<Option<Block>, RpcError>  

// Get block header only 
pub async fn get_block_header_by_number(  
    &self,  
    number: u64,  
    full_tx: bool  
) -> Result<Option<BlockHeader>, RpcError>
``` 

### Transactions


```// Get transaction by hash 
pub async fn get_transaction_by_tx_hash(  
    &self,  
    hash: B256  
) -> Result<Option<TransactionTx>, RpcError>  

// Get transaction by block and index 
pub async fn get_transaction_by_block_with_index(  
    &self,  
    block: BlockIdentifier,  
    index: U64  
) -> Result<Option<TransactionTx>, RpcError>
``` 

### State

```// Get account balance 
pub async fn get_balance(  
    &self,  
    address: Address,  
    block: BlockNumber  
) -> Result<U256, RpcError>  

// Get contract code 
pub async fn get_code(  
    &self,  
    address: Address,  
    block: BlockNumber  
) -> Result<Bytes, RpcError>
``` 

### Logs

```// Get event logs in block range with optional filtering 
pub async fn get_logs(  
    &self,  
    from_block: u64,  
    to_block: u64,  
    address: Option<Address>,  
    topics: Option<Vec<B256>>  
) -> Result<Option<Vec<Log>>, RpcError>
``` 

----------

## Performance Highlights

-   **Zero Allocation Parsing**: Faster and memory efficient.
-   **Benchmarks**:
    -   **MacBook Air M3**:
        -   **1000 Concurrent Requests for blocks**: 431ms.
        -   **Log Fetching**: ~95% faster than standard implementations.
        -   **Block Processing**: ~2300 blocks/second.
    -   Performance varies depending on hardware.

----------

## Usage Example


```let transport = HttpTransport::new(url);  
let client = RpcClient::new(transport);  

// Fetch blocks concurrently 
let blocks = client.get_block_by_number(block_num, false).await?;
``` 

----------

## Contribution

Contributions are welcome! Please submit issues or pull requests to help improve this library.

## License

This project is licensed under MIT License.