use palantiri::{hyper_rpc::RpcClient as HyperRpcClient, transport::http::TransportBuilder};
use std::time::Instant;
use tokio;

const RPC_URL: &str = "https://thrilling-boldest-panorama.quiknode.pro/c11ea3b6cfa7edd1abd7d29d66cc2f268cc11515/";
const TEST_BLOCK: u64 = 23330565;

#[tokio::main]
async fn main() {
    println!("Testing HyperTransport minimal with connection pooling...");
    
    // Test HyperTransport minimal (should work and be fast)
    println!("\n=== Testing HyperTransport Minimal ===");
    let hyper_client = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_http_hyper_minimal());
    
    // Make multiple requests to see connection reuse benefit
    for i in 1..=5 {
        let start = Instant::now();
        match hyper_client.get_block_by_number(TEST_BLOCK, true).await {
            Ok(_) => println!("HyperTransport Minimal request {}: SUCCESS in {:?}", i, start.elapsed()),
            Err(e) => println!("HyperTransport Minimal request {} FAILED: {}", i, e),
        }
    }
    
    // Compare with fresh clients
    println!("\n=== Testing DirectReqwestTransport (fresh clients) ===");
    for i in 1..=3 {
        let start = Instant::now();
        let fresh_client = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_direct_reqwest());
        match fresh_client.get_block_by_number(TEST_BLOCK, true).await {
            Ok(_) => println!("DirectReqwest request {}: SUCCESS in {:?}", i, start.elapsed()),
            Err(e) => println!("DirectReqwest request {} FAILED: {}", i, e),
        }
    }
}