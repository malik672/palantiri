use palantiri::{hyper_rpc::RpcClient as HyperRpcClient, transport::http::TransportBuilder};
use std::time::Instant;
use tokio;

const RPC_URL: &str = "https://thrilling-boldest-panorama.quiknode.pro/c11ea3b6cfa7edd1abd7d29d66cc2f268cc11515/";
const TEST_BLOCK: u64 = 23330565;

#[tokio::main]
async fn main() {
    println!("Testing ALL transports to find working ones...");
    
    // Test ReqwestTransport first (should work)
    println!("\n=== Testing ReqwestTransport ===");
    let reqwest_client = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_reqwest());
    let start = Instant::now();
    match reqwest_client.get_block_by_number(TEST_BLOCK, true).await {
        Ok(_) => println!("ReqwestTransport: SUCCESS in {:?}", start.elapsed()),
        Err(e) => println!("ReqwestTransport: FAILED - {}", e),
    }
    
    // Test minimal hyper transport
    println!("\n=== Testing HyperTransport Minimal ===");
    let hyper_client = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_http_hyper_minimal());
    let start = Instant::now();
    match hyper_client.get_block_by_number(TEST_BLOCK, true).await {
        Ok(_) => println!("HyperTransport Minimal: SUCCESS in {:?}", start.elapsed()),
        Err(e) => println!("HyperTransport Minimal: FAILED - {}", e),
    }
    
    // Test DirectTransport
    println!("\n=== Testing DirectTransport ===");
    let direct_client = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_direct());
    let start = Instant::now();
    match direct_client.get_block_by_number(TEST_BLOCK, true).await {
        Ok(_) => println!("DirectTransport: SUCCESS in {:?}", start.elapsed()),
        Err(e) => println!("DirectTransport: FAILED - {}", e),
    }
}