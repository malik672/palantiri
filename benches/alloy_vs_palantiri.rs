use std::time::Duration;

use ::palantiri::{hyper_rpc::RpcClient as HyperRpcClient, transport::http::TransportBuilder};
use alloy::{
    eips::BlockNumberOrTag,
    providers::{Provider, ProviderBuilder},
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;

// Test with recent blocks where network optimizations matter most
const RECENT_BLOCKS: [u64; 5] = [23334905, 23334696,23334696,23334696,23334696]; // Current blocks
const OLD_BLOCKS: [u64; 5] =  [23334905, 23334696,23334696,23334696,23334696]; // Current blocks
const RPC_URL: &str = "https://thrilling-boldest-panorama.quiknode.pro/c11ea3b6cfa7edd1abd7d29d66cc2f268cc11515/";

pub fn benchmark_single_block_recent(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Setup clients
    let rpc_url = RPC_URL.parse().unwrap();
    let alloy_provider = ProviderBuilder::new().on_http(rpc_url);

    let mut group = c.benchmark_group("single_block_recent");
    group.sample_size(15);
    group.measurement_time(Duration::from_secs(60));

    group.bench_function("alloy_recent", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = alloy_provider
                    .get_block_by_number(BlockNumberOrTag::Number(RECENT_BLOCKS[0]))
                    .await;
                black_box(result)
            })
        });
    });

    // Test Palantiri with ultra-fast Reqwest to beat Alloy's 194ms
    let palantiri_ultra_fast = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_reqwest_ultra_fast());

    group.bench_function("palantiri_ultra_fast", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = palantiri_ultra_fast.get_block_by_number(RECENT_BLOCKS[0], true).await;
                black_box(result)
            })
        });
    });

    // Test Palantiri with optimized Reqwest (connection pooling + HTTP/2)
    let palantiri_optimized = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_reqwest_optimized());
    
    group.bench_function("palantiri_optimized", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = palantiri_optimized.get_block_by_number(RECENT_BLOCKS[0], true).await;
                black_box(result)
            })
        });
    });

    // Test standard Reqwest for comparison  
    let palantiri_reqwest = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_reqwest());
    
    group.bench_function("palantiri_reqwest", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = palantiri_reqwest.get_block_by_number(RECENT_BLOCKS[0], true).await;
                black_box(result)
            })
        });
    });

    group.finish();
}

pub fn benchmark_single_block_old(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Setup clients
    let rpc_url = RPC_URL.parse().unwrap();
    let alloy_provider = ProviderBuilder::new().on_http(rpc_url);
    let palantiri_rpc = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_reqwest());

    let mut group = c.benchmark_group("single_block_old");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("alloy_old", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = alloy_provider
                    .get_block_by_number(BlockNumberOrTag::Number(OLD_BLOCKS[0]))
                    .await;
                black_box(result)
            })
        });
    });

    group.bench_function("palantiri_old", |b| {
        b.iter(|| {
            rt.block_on(async {
                
                let result = palantiri_rpc.get_block_by_number(OLD_BLOCKS[0], true).await;
                black_box(result)
            })
        });
    });

    group.finish();
}

pub fn benchmark_multiple_blocks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Setup clients
    let rpc_url = RPC_URL.parse().unwrap();
    let alloy_provider = ProviderBuilder::new().on_http(rpc_url);

    let palantiri_rpc = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_http_hyper_benchmark());

    let mut group = c.benchmark_group("multiple_blocks");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(120));

    // Alloy - individual requests (sequential)
    group.bench_function("alloy_sequential_recent", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut blocks = Vec::new();
                for &block_num in &RECENT_BLOCKS {
                    match alloy_provider
                        .get_block_by_number(BlockNumberOrTag::Number(block_num))
                        .await
                    {
                        Ok(block) => blocks.push(Some(block)),
                        Err(_) => blocks.push(None),
                    }
                }
                black_box(blocks)
            })
        });
    });

    // Alloy - concurrent requests
    group.bench_function("alloy_concurrent_recent", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = Vec::new();
                for &block_num in &RECENT_BLOCKS {
                    let provider = alloy_provider.clone();
                    let handle = tokio::spawn(async move {
                        provider.get_block_by_number(BlockNumberOrTag::Number(block_num)).await
                    });
                    handles.push(handle);
                }

                let results = futures::future::join_all(handles).await;
                black_box(results)
            })
        });
    });

    // Palantiri - individual requests
    group.bench_function("palantiri_sequential_recent", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut blocks = Vec::new();
                for &block_num in &RECENT_BLOCKS {
                    match palantiri_rpc.get_block_by_number(block_num, true).await {
                        Ok(block) => blocks.push(block),
                        Err(_) => blocks.push(None),
                    }
                }
                black_box(blocks)
            })
        });
    });

    // Palantiri - BATCHED requests (our advantage!)
    group.bench_function("palantiri_batch_recent", |b| {
        b.iter(|| {
            rt.block_on(async {
                let blocks =
                    palantiri_rpc.get_blocks_by_numbers(RECENT_BLOCKS.to_vec(), true).await;
                black_box(blocks)
            })
        });
    });

    group.finish();
}

pub fn benchmark_caching_advantage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Setup clients
    let rpc_url = RPC_URL.parse().unwrap();
    let alloy_provider = ProviderBuilder::new().on_http(rpc_url);

    let palantiri_rpc = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_http_hyper_benchmark());

    // Pre-warm Palantiri cache
    rt.block_on(async {
        let _ = palantiri_rpc.get_block_by_number(RECENT_BLOCKS[0], true).await;
    });

    let mut group = c.benchmark_group("caching_advantage");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(20));

    // Alloy - no caching
    group.bench_function("alloy_no_cache", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = alloy_provider
                    .get_block_by_number(BlockNumberOrTag::Number(RECENT_BLOCKS[0]))
                    .await;
                black_box(result)
            })
        });
    });

    // Palantiri - with caching
    group.bench_function("palantiri_cached", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = palantiri_rpc.get_block_by_number(RECENT_BLOCKS[0], true).await;
                black_box(result)
            })
        });
    });

    group.finish();
}

pub fn benchmark_tower_transport(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Setup clients
    let rpc_url = RPC_URL.parse().unwrap();
    let alloy_provider = ProviderBuilder::new().on_http(rpc_url);

    let palantiri_hyper = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_http_hyper_minimal());
    let palantiri_tower = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_tower());
    let palantiri_tower_optimized = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_tower_optimized());

    let mut group = c.benchmark_group("tower_transport_comparison");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(60));

    group.bench_function("alloy_baseline", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = alloy_provider
                    .get_block_by_number(BlockNumberOrTag::Number(RECENT_BLOCKS[0]))
                    .await;
                black_box(result)
            })
        });
    });

    group.bench_function("palantiri_hyper_minimal", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = palantiri_hyper.get_block_by_number(RECENT_BLOCKS[0], true).await;
                black_box(result)
            })
        });
    });

    group.bench_function("palantiri_tower", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = palantiri_tower.get_block_by_number(RECENT_BLOCKS[0], true).await;
                black_box(result)
            })
        });
    });

    group.bench_function("palantiri_tower_optimized", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = palantiri_tower_optimized.get_block_by_number(RECENT_BLOCKS[0], true).await;
                black_box(result)
            })
        });
    });

    group.finish();
}

pub fn benchmark_tower_batch(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let palantiri_hyper = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_http_hyper_minimal());
    let palantiri_tower_optimized = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_tower_optimized());

    let mut group = c.benchmark_group("tower_batch_comparison");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("hyper_batch", |b| {
        b.iter(|| {
            rt.block_on(async {
                let blocks = palantiri_hyper.get_blocks_by_numbers(RECENT_BLOCKS.to_vec(), true).await;
                black_box(blocks)
            })
        });
    });

    group.bench_function("tower_batch", |b| {
        b.iter(|| {
            rt.block_on(async {
                let blocks = palantiri_tower_optimized.get_blocks_by_numbers(RECENT_BLOCKS.to_vec(), true).await;
                black_box(blocks)
            })
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_single_block_recent,
    benchmark_single_block_old,
    benchmark_multiple_blocks,
    benchmark_caching_advantage,
    benchmark_tower_transport,
    benchmark_tower_batch,
);
criterion_main!(benches);
