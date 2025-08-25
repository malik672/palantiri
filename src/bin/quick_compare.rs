use palantiri::{transport::http::TransportBuilder, hyper_rpc::RpcClient as HyperRpcClient};
use alloy::{eips::BlockNumberOrTag, providers::{Provider, ProviderBuilder}};
use std::time::Instant;
use tokio;

const OLD_BLOCK: u64 = 22812202;
const RECENT_BLOCK: u64 = 23218929;
const RPC_URL: &str = "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4";

#[tokio::main]
async fn main() {
    println!("🔥 Alloy vs Palantiri Performance Comparison");
    
    // Setup clients
    let rpc_url = RPC_URL.parse().unwrap();
    let alloy_provider = ProviderBuilder::new().on_http(rpc_url);
    
    let palantiri_rpc = HyperRpcClient::new(
        TransportBuilder::new(RPC_URL).build_http_hyper(),
    );
    
    println!("\n📊 Testing OLD block ({})", OLD_BLOCK);
    
    // Test 1: Old block - Alloy
    let start = Instant::now();
    match alloy_provider.get_block_by_number(BlockNumberOrTag::Number(OLD_BLOCK)).await {
        Ok(block) => {
            let alloy_time = start.elapsed();
            println!("  ✅ Alloy: {:?} (block exists: {})", alloy_time, block.is_some());
        }
        Err(e) => println!("  ❌ Alloy error: {}", e),
    }
    
    // Test 2: Old block - Palantiri
    let start = Instant::now();
    match palantiri_rpc.get_block_by_number(OLD_BLOCK, true).await {
        Ok(block) => {
            let palantiri_time = start.elapsed();
            println!("  ✅ Palantiri: {:?} (block exists: {})", palantiri_time, block.is_some());
        }
        Err(e) => println!("  ❌ Palantiri error: {}", e),
    }
    
    println!("\n📊 Testing RECENT block ({}) - expect slower times", RECENT_BLOCK);
    
    // Test 3: Recent block - Alloy
    let start = Instant::now();
    match alloy_provider.get_block_by_number(BlockNumberOrTag::Number(RECENT_BLOCK)).await {
        Ok(block) => {
            let alloy_time = start.elapsed();
            println!("  ✅ Alloy: {:?} (block exists: {})", alloy_time, block.is_some());
        }
        Err(e) => println!("  ❌ Alloy error: {}", e),
    }
    
    // Test 4: Recent block - Palantiri (first time - no cache)
    let start = Instant::now();
    match palantiri_rpc.get_block_by_number(RECENT_BLOCK, true).await {
        Ok(block) => {
            let palantiri_time = start.elapsed();
            println!("  ✅ Palantiri (uncached): {:?} (block exists: {})", palantiri_time, block.is_some());
        }
        Err(e) => println!("  ❌ Palantiri error: {}", e),
    }
    
    // Test 5: Recent block - Palantiri (second time - should be cached!)
    let start = Instant::now();
    match palantiri_rpc.get_block_by_number(RECENT_BLOCK, true).await {
        Ok(block) => {
            let palantiri_cached_time = start.elapsed();
            println!("  🚀 Palantiri (cached): {:?} (block exists: {})", palantiri_cached_time, block.is_some());
        }
        Err(e) => println!("  ❌ Palantiri cached error: {}", e),
    }
    
    println!("\n🎯 Testing BATCH functionality (Palantiri advantage)");
    
    let batch_blocks = vec![OLD_BLOCK, OLD_BLOCK - 1, OLD_BLOCK - 2, OLD_BLOCK - 3, OLD_BLOCK - 4];
    
    // Test 6: Multiple blocks - Alloy sequential
    let start = Instant::now();
    let mut alloy_blocks = Vec::new();
    for &block_num in &batch_blocks {
        match alloy_provider.get_block_by_number(BlockNumberOrTag::Number(block_num)).await {
            Ok(block) => alloy_blocks.push(block),
            Err(_) => alloy_blocks.push(None),
        }
    }
    let alloy_sequential_time = start.elapsed();
    println!("  ✅ Alloy Sequential (5 blocks): {:?}", alloy_sequential_time);
    
    // Test 7: Multiple blocks - Palantiri batch
    let start = Instant::now();
    match palantiri_rpc.get_blocks_by_numbers(batch_blocks.clone(), true).await {
        Ok(blocks) => {
            let palantiri_batch_time = start.elapsed();
            println!("  🚀 Palantiri Batch (5 blocks): {:?} (got {} blocks)", 
                palantiri_batch_time, blocks.len());
        }
        Err(e) => println!("  ❌ Palantiri batch error: {}", e),
    }
    
    println!("\n🏆 Summary: Look for caching speedup and batch advantages!");
}