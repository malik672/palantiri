use palantiri::{hyper_rpc::RpcClient as HyperRpcClient, transport::http::TransportBuilder};
use std::time::Instant;
use tokio;

const RPC_URL: &str = "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4";
const TEST_BLOCK: u64 = 23326322;

#[tokio::main]
async fn main() {
    println!("Testing Palantiri performance...");
    
    // Test with minimal transport (connection pooling)
    println!("\n=== Testing with minimal transport (connection pooling) ===");
    let palantiri_minimal = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_http_hyper_minimal());
    
    for i in 1..=5 {
        let start = Instant::now();
        let result = palantiri_minimal.get_block_by_number(TEST_BLOCK, true).await;
        let duration = start.elapsed();
        
        match result {
            Ok(_) => println!("Request {}: {:?}", i, duration),
            Err(e) => println!("Request {} failed: {}", i, e),
        }
    }
    
    // Test with benchmark transport (minimal pooling)
    println!("\n=== Testing with benchmark transport (minimal pooling) ===");
    let palantiri_bench = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_http_hyper_benchmark());
    
    for i in 1..=5 {
        let start = Instant::now();
        let result = palantiri_bench.get_block_by_number(TEST_BLOCK, true).await;
        let duration = start.elapsed();
        
        match result {
            Ok(_) => println!("Request {}: {:?}", i, duration),
            Err(e) => println!("Request {} failed: {}", i, e),
        }
    }
    
    // Test with fresh client each time (like the broken benchmark)
    println!("\n=== Testing with fresh client each time ===");
    for i in 1..=3 {
        let start = Instant::now();
        let fresh_client = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_reqwest());
        let result = fresh_client.get_block_by_number(TEST_BLOCK, true).await;
        let duration = start.elapsed();
        
        match result {
            Ok(_) => println!("Request {}: {:?}", i, duration),
            Err(e) => println!("Request {} failed: {}", i, e),
        }
    }
}