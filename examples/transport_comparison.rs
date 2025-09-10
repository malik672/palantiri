use palantiri::{hyper_rpc::RpcClient as HyperRpcClient, transport::http::TransportBuilder};
use std::time::Instant;
use tokio;

const RPC_URL: &str = "https://thrilling-boldest-panorama.quiknode.pro/c11ea3b6cfa7edd1abd7d29d66cc2f268cc11515/";
const TEST_BLOCK: u64 = 23330565; // 0x1640b05 - current block

#[tokio::main]
async fn main() {
    println!("Testing ALL transport implementations to see which ones work...");
    
    // Test reqwest transport (this should work)
    println!("\n=== Testing ReqwestTransport ===");
    let reqwest_client = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_reqwest());
    let start = Instant::now();
    match reqwest_client.get_block_by_number(TEST_BLOCK, true).await {
        Ok(_) => println!("ReqwestTransport: SUCCESS in {:?}", start.elapsed()),
        Err(e) => println!("ReqwestTransport: FAILED - {}", e),
    }
    
    // Test minimal reqwest transport
    println!("\n=== Testing ReqwestTransport Minimal ===");
    let reqwest_minimal_client = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_reqwest_minimal());
    let start = Instant::now();
    match reqwest_minimal_client.get_block_by_number(TEST_BLOCK, true).await {
        Ok(_) => println!("ReqwestTransport Minimal: SUCCESS in {:?}", start.elapsed()),
        Err(e) => println!("ReqwestTransport Minimal: FAILED - {}", e),
    }
    
    // Test hyper transport minimal
    println!("\n=== Testing HyperTransport Minimal ===");
    let hyper_minimal_client = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_http_hyper_minimal());
    let start = Instant::now();
    match hyper_minimal_client.get_block_by_number(TEST_BLOCK, true).await {
        Ok(_) => println!("HyperTransport Minimal: SUCCESS in {:?}", start.elapsed()),
        Err(e) => println!("HyperTransport Minimal: FAILED - {}", e),
    }
    
    // Test direct transport
    println!("\n=== Testing DirectTransport ===");
    let direct_client = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_direct());
    let start = Instant::now();
    match direct_client.get_block_by_number(TEST_BLOCK, true).await {
        Ok(_) => println!("DirectTransport: SUCCESS in {:?}", start.elapsed()),
        Err(e) => println!("DirectTransport: FAILED - {}", e),
    }
}