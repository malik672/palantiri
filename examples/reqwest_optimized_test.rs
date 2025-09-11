use palantiri::{hyper_rpc::RpcClient as HyperRpcClient, transport::http::TransportBuilder};
use std::time::Instant;
use tokio;

const RPC_URL: &str = "https://thrilling-boldest-panorama.quiknode.pro/c11ea3b6cfa7edd1abd7d29d66cc2f268cc11515/";
const TEST_BLOCK: u64 = 23330565;

#[tokio::main]
async fn main() {
    println!("Testing optimized Reqwest transport to beat Alloy...");
    
    // Test optimized Reqwest (HTTP/2 + connection pooling)
    println!("\n=== Testing Optimized ReqwestTransport ===");
    let optimized_client = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_reqwest_optimized());
    
    // Make multiple requests to see connection reuse benefit
    for i in 1..=5 {
        let start = Instant::now();
        match optimized_client.get_block_by_number(TEST_BLOCK, true).await {
            Ok(_) => println!("Optimized Reqwest request {}: SUCCESS in {:?}", i, start.elapsed()),
            Err(e) => println!("Optimized Reqwest request {} FAILED: {}", i, e),
        }
    }
    
    // Compare with standard Reqwest
    println!("\n=== Testing Standard ReqwestTransport ===");
    let standard_client = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_reqwest());
    
    for i in 1..=3 {
        let start = Instant::now();
        match standard_client.get_block_by_number(TEST_BLOCK, true).await {
            Ok(_) => println!("Standard Reqwest request {}: SUCCESS in {:?}", i, start.elapsed()),
            Err(e) => println!("Standard Reqwest request {} FAILED: {}", i, e),
        }
    }
    
    println!("\n=== Target: Beat Alloy's 182ms average ===");
}