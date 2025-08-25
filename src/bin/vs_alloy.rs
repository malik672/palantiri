use palantiri::{transport::http::TransportBuilder, hyper_rpc::RpcClient as HyperRpcClient};
use alloy::{eips::BlockNumberOrTag, providers::{Provider, ProviderBuilder}};
use std::time::Instant;
use tokio;

const TEST_BLOCKS: [u64; 3] = [22812202, 22812201, 22812200]; // Older, more stable blocks
const RPC_URL: &str = "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4";

#[tokio::main]
async fn main() {
    println!("⚡ Alloy vs Optimized Palantiri Comparison");
    
    // Setup clients
    let rpc_url = RPC_URL.parse().unwrap();
    let alloy_provider = ProviderBuilder::new().on_http(rpc_url);
    
    let palantiri_rpc = HyperRpcClient::new(
        TransportBuilder::new(RPC_URL).build_http_hyper(),
    );
    
    println!("\n🔥 Round 1: Single Block Performance");
    
    // Test 1: Single block - Alloy
    let start = Instant::now();
    match alloy_provider.get_block_by_number(BlockNumberOrTag::Number(TEST_BLOCKS[0])).await {
        Ok(_) => {
            let alloy_single = start.elapsed();
            println!("  📊 Alloy single block: {:?}", alloy_single);
        }
        Err(e) => println!("  ❌ Alloy error: {}", e),
    }
    
    // Test 2: Single block - Palantiri  
    let start = Instant::now();
    match palantiri_rpc.get_block_by_number(TEST_BLOCKS[0], true).await {
        Ok(_) => {
            let palantiri_single = start.elapsed();
            println!("  📊 Palantiri single block: {:?}", palantiri_single);
        }
        Err(e) => println!("  ❌ Palantiri error: {}", e),
    }
    
    println!("\n🚀 Round 2: Multiple Block Performance (Palantiri's Strength)");
    
    // Test 3: Multiple blocks - Alloy (individual requests)
    let start = Instant::now();
    let mut alloy_blocks = Vec::new();
    for &block_num in &TEST_BLOCKS {
        match alloy_provider.get_block_by_number(BlockNumberOrTag::Number(block_num)).await {
            Ok(block) => alloy_blocks.push(block),
            Err(_) => alloy_blocks.push(None),
        }
    }
    let alloy_multiple = start.elapsed();
    println!("  📊 Alloy {} blocks (sequential): {:?}", TEST_BLOCKS.len(), alloy_multiple);
    
    // Test 4: Multiple blocks - Alloy (concurrent)
    let start = Instant::now();
    let mut handles = Vec::new();
    for &block_num in &TEST_BLOCKS {
        let provider = alloy_provider.clone();
        let handle = tokio::spawn(async move {
            provider.get_block_by_number(BlockNumberOrTag::Number(block_num)).await
        });
        handles.push(handle);
    }
    let _results = futures::future::join_all(handles).await;
    let alloy_concurrent = start.elapsed();
    println!("  📊 Alloy {} blocks (concurrent): {:?}", TEST_BLOCKS.len(), alloy_concurrent);
    
    // Test 5: Multiple blocks - Palantiri (individual requests)
    let start = Instant::now();
    let mut palantiri_blocks = Vec::new();
    for &block_num in &TEST_BLOCKS {
        match palantiri_rpc.get_block_by_number(block_num, true).await {
            Ok(block) => palantiri_blocks.push(block),
            Err(_) => palantiri_blocks.push(None),
        }
    }
    let palantiri_individual = start.elapsed();
    println!("  📊 Palantiri {} blocks (individual): {:?}", TEST_BLOCKS.len(), palantiri_individual);
    
    // Test 6: Multiple blocks - Palantiri (BATCHED - our secret weapon!)
    let start = Instant::now();
    match palantiri_rpc.get_blocks_by_numbers(TEST_BLOCKS.to_vec(), true).await {
        Ok(blocks) => {
            let palantiri_batch = start.elapsed();
            println!("  🚀 Palantiri {} blocks (BATCH): {:?} [got {} blocks]", 
                TEST_BLOCKS.len(), palantiri_batch, blocks.len());
        }
        Err(e) => println!("  ❌ Palantiri batch error: {}", e),
    }
    
    println!("\n💾 Round 3: Caching Performance (Palantiri's Secret Weapon)");
    
    // Pre-warm Palantiri cache
    let _ = palantiri_rpc.get_block_by_number(TEST_BLOCKS[0], true).await;
    
    // Test 7: Alloy repeated request (no caching)
    let start = Instant::now();
    let _ = alloy_provider.get_block_by_number(BlockNumberOrTag::Number(TEST_BLOCKS[0])).await;
    let alloy_repeat = start.elapsed();
    println!("  📊 Alloy repeated request: {:?}", alloy_repeat);
    
    // Test 8: Palantiri repeated request (cached!)
    let start = Instant::now();
    let _ = palantiri_rpc.get_block_by_number(TEST_BLOCKS[0], true).await;
    let palantiri_cached = start.elapsed();
    println!("  🚀 Palantiri cached request: {:?}", palantiri_cached);
    
    if palantiri_cached.as_millis() < 50 {
        println!("  🎯 Cache HIT! (sub-50ms indicates cached response)");
    }
    
    println!("\n🏆 SUMMARY:");
    println!("  • Single requests: Expect similar performance");
    println!("  • Multiple requests: Batching should give Palantiri advantage");
    println!("  • Repeated requests: Caching should make Palantiri MUCH faster");
    println!("  • Best use case: Applications that query recent blocks repeatedly");
}