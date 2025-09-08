use ::palantiri::{transport::http::TransportBuilder, hyper_rpc::RpcClient as HyperRpcClient};
use alloy::{eips::BlockNumberOrTag, providers::{Provider, ProviderBuilder}};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;
use std::{sync::{atomic::{AtomicU64, Ordering}}, time::Duration};

// Standard benchmark configuration
const SAMPLE_SIZE: usize = 20;
const MEASUREMENT_TIME: Duration = Duration::from_secs(20);
const STARTING_BLOCK: u64 = 21000000;
const RPC_URL: &str = "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4";

pub fn benchmark_alloy(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let rpc_url = RPC_URL.parse().unwrap();
    let provider = ProviderBuilder::new().on_http(rpc_url);
    
    let block_counter = AtomicU64::new(STARTING_BLOCK);
    
    let mut group = c.benchmark_group("ethereum_rpc_comparison");
    group.sample_size(SAMPLE_SIZE);
    group.measurement_time(MEASUREMENT_TIME);
    
    group.bench_function("alloy_get_block", |b| {
        b.iter(|| {
            rt.block_on(async {
                let block_num = block_counter.fetch_sub(1, Ordering::SeqCst);
                let result = provider
                    .get_block_by_number(BlockNumberOrTag::Number(block_num))
                    .await;
                
                match result {
                    Ok(block) => black_box(block),
                    Err(e) => {
                        eprintln!("Alloy error fetching block {}: {}", block_num, e);
                        black_box(None)
                    }
                }
            })
        });
    });
    
    group.finish();
}

pub fn benchmark_palantiri(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let rpc = HyperRpcClient::new(
        TransportBuilder::new(RPC_URL)
            .build_http_hyper(),
    );
    
    let block_counter = AtomicU64::new(STARTING_BLOCK);
    
    let mut group = c.benchmark_group("ethereum_rpc_comparison");
    group.sample_size(SAMPLE_SIZE);
    group.measurement_time(MEASUREMENT_TIME);
    
    group.bench_function("palantiri_get_block", |b| {
        b.iter(|| {
            rt.block_on(async {
                let block_num = block_counter.fetch_sub(1, Ordering::SeqCst);
                let result = rpc.get_block_by_number(block_num, true).await;
                black_box(result)
            })
        });
    });
    
    group.finish();
}

pub fn benchmark_concurrent_alloy(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_requests");
    group.sample_size(10);
    group.measurement_time(MEASUREMENT_TIME);
    
    group.bench_function("alloy_concurrent_10", |b| {
        b.iter(|| {
            rt.block_on(async {
                let rpc_url = RPC_URL.parse().unwrap();
                let provider = ProviderBuilder::new().on_http(rpc_url);
                
                let mut handles = Vec::new();
                let start_block = STARTING_BLOCK;
                
                for i in 0..10 {
                    let provider = provider.clone();
                    let handle = tokio::spawn(async move {
                        provider.get_block_by_number(BlockNumberOrTag::Number(start_block - i)).await
                    });
                    handles.push(handle);
                }
                
                let results = futures::future::join_all(handles).await;
                black_box(results)
            })
        });
    });
    
    group.finish();
}

pub fn benchmark_concurrent_palantiri(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_requests");
    group.sample_size(10);
    group.measurement_time(MEASUREMENT_TIME);
    
    group.bench_function("palantiri_concurrent_10", |b| {
        b.iter(|| {
            rt.block_on(async {
                let rpc = HyperRpcClient::new(
                    TransportBuilder::new(RPC_URL)
                        .build_http_hyper(),
                );
                
                let mut handles = Vec::new();
                let start_block = STARTING_BLOCK;
                
                for i in 0..10 {
                    let rpc = rpc.clone();
                    let handle = tokio::spawn(async move {
                        rpc.get_block_by_number(start_block - i, true).await
                    });
                    handles.push(handle);
                }
                
                let results = futures::future::join_all(handles).await;
                black_box(results)
            })
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_alloy,
    benchmark_palantiri,
    benchmark_concurrent_alloy,
    benchmark_concurrent_palantiri,
);
criterion_main!(benches);
