use palantiri::{transport::http::TransportBuilder, hyper_rpc::Transport};
use std::time::Instant;
use tokio;

const RPC_URL: &str = "https://thrilling-boldest-panorama.quiknode.pro/c11ea3b6cfa7edd1abd7d29d66cc2f268cc11515/";

#[tokio::main]
async fn main() {
    println!("Testing DirectTransport at the transport level...");
    
    // Create DirectTransport
    let direct_transport = TransportBuilder::new(RPC_URL).build_direct();
    
    // Create a simple JSON-RPC request
    let request = r#"{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}"#;
    
    println!("Making direct transport request...");
    let start = Instant::now();
    
    match direct_transport.hyper_execute(request.to_string()).await {
        Ok(response) => {
            println!("DirectTransport SUCCESS in {:?}", start.elapsed());
            println!("Response: {}", response);
        },
        Err(e) => {
            println!("DirectTransport FAILED: {}", e);
        }
    }
    
    // Test DirectReqwestTransport 
    println!("\n=== Testing DirectReqwestTransport ===");
    let direct_reqwest_transport = TransportBuilder::new(RPC_URL).build_direct_reqwest();
    
    let start = Instant::now();
    match direct_reqwest_transport.hyper_execute(request.to_string()).await {
        Ok(response) => {
            println!("DirectReqwestTransport SUCCESS in {:?}", start.elapsed());
            println!("Response: {}", response);
        },
        Err(e) => {
            println!("DirectReqwestTransport FAILED: {}", e);
        }
    }
    
    // Test ReqwestTransport for comparison
    println!("\n=== Testing ReqwestTransport for comparison ===");
    let reqwest_transport = TransportBuilder::new(RPC_URL).build_reqwest();
    
    let start = Instant::now();
    match reqwest_transport.hyper_execute(request.to_string()).await {
        Ok(response) => {
            println!("ReqwestTransport SUCCESS in {:?}", start.elapsed());
            println!("Response: {}", response);
        },
        Err(e) => {
            println!("ReqwestTransport FAILED: {}", e);
        }
    }
}