use palantiri::{transport::http::TransportBuilder, hyper_rpc::RpcClient as HyperRpcClient};
use std::time::Instant;
use tokio;

const RECENT_BLOCK: u64 = 23218929;
const OLD_BLOCK: u64 = 22812202;
const RPC_URL: &str = "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4";

#[tokio::main]
async fn main() {
    println!("🔍 Testing Network vs Parsing Performance");
    
    let rpc = HyperRpcClient::new(
        TransportBuilder::new(RPC_URL).build_http_hyper(),
    );
    
    // Test 1: Old block network timing
    println!("\n📊 Testing OLD block ({})", OLD_BLOCK);
    let start = Instant::now();
    let old_request = palantiri::hyper_rpc::RpcRequest {
        jsonrpc: "2.0",
        method: "eth_getBlockByNumber",
        params: serde_json::json!([format!("0x{:x}", OLD_BLOCK), true]),
        id: 1,
    };
    
    match rpc.execute_raw(old_request).await {
        Ok(data) => {
            let network_time = start.elapsed();
            println!("  ✅ Network time: {:?}", network_time);
            println!("  📦 Response size: {} bytes", data.len());
            
            // Test parsing time
            let parse_start = Instant::now();
            let parsed = palantiri::parser::block_parser::parse_block(&data);
            let parse_time = parse_start.elapsed();
            println!("  ⚡ Parse time: {:?}", parse_time);
            println!("  📈 Parsing success: {}", parsed.is_some());
        }
        Err(e) => println!("  ❌ Error: {}", e),
    }
    
    // Test 2: Recent block network timing  
    println!("\n📊 Testing RECENT block ({})", RECENT_BLOCK);
    let start = Instant::now();
    let recent_request = palantiri::hyper_rpc::RpcRequest {
        jsonrpc: "2.0",
        method: "eth_getBlockByNumber", 
        params: serde_json::json!([format!("0x{:x}", RECENT_BLOCK), true]),
        id: 1,
    };
    
    match rpc.execute_raw(recent_request).await {
        Ok(data) => {
            let network_time = start.elapsed();
            println!("  ✅ Network time: {:?}", network_time);
            println!("  📦 Response size: {} bytes", data.len());
            
            // Test parsing time
            let parse_start = Instant::now();
            let parsed = palantiri::parser::block_parser::parse_block(&data);
            let parse_time = parse_start.elapsed();
            println!("  ⚡ Parse time: {:?}", parse_time);
            println!("  📈 Parsing success: {}", parsed.is_some());
        }
        Err(e) => println!("  ❌ Error: {}", e),
    }
    
    println!("\n🎯 Conclusion: Compare network times to identify the bottleneck!");
}