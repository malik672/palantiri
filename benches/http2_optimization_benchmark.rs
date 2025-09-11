use std::time::Duration;

use ::palantiri::{hyper_rpc::RpcClient as HyperRpcClient, transport::http::TransportBuilder};
use alloy::{eips::BlockNumberOrTag, providers::{Provider, ProviderBuilder}};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;

const RPC_URL: &str = "https://ethereum-rpc.publicnode.com";
const TEST_BLOCKS: [u64; 3] = [23326322, 23326321, 23326320];

pub fn benchmark_http2_optimizations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    // Setup clients
    let rpc_url = RPC_URL.parse().unwrap();
    let alloy_provider = ProviderBuilder::new().on_http(rpc_url);
    
    // Original Palantiri client with HTTP/2 enabled
    let palantiri_original = HyperRpcClient::new(
        TransportBuilder::new(RPC_URL).build_http_hyper(),
    );
    
    // New minimal Alloy-style client 
    let palantiri_minimal = HyperRpcClient::new(
        TransportBuilder::new(RPC_URL).build_http_hyper_minimal(),
    );
    
    let mut group = c.benchmark_group("http2_optimization");
    group.sample_size(5);
    group.measurement_time(Duration::from_secs(30));
    
    group.bench_function("alloy_baseline", |b| {
        b.iter(|| {
            rt.block_on(async {
                for block_num in TEST_BLOCKS {
                    let block = alloy_provider
                        .get_block_by_number(BlockNumberOrTag::Number(block_num))
                        .await
                        .unwrap();
                    black_box(block);
                }
            })
        });
    });
    
    group.bench_function("palantiri_original_http2", |b| {
        b.iter(|| {
            rt.block_on(async {
                for block_num in TEST_BLOCKS {
                    let block = palantiri_original
                        .get_block_by_number(block_num, false)
                        .await
                        .unwrap();
                    black_box(block);
                }
            })
        });
    });
    
    group.bench_function("palantiri_minimal_alloy_style", |b| {
        b.iter(|| {
            rt.block_on(async {
                for block_num in TEST_BLOCKS {
                    let block = palantiri_minimal
                        .get_block_by_number(block_num, false)
                        .await
                        .unwrap();
                    black_box(block);
                }
            })
        });
    });
    
    group.finish();
}

pub fn benchmark_single_request_optimization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let palantiri_minimal = HyperRpcClient::new(
        TransportBuilder::new(RPC_URL).build_http_hyper_minimal(),
    );
    
    let mut group = c.benchmark_group("single_request_optimization");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));
    
    group.bench_function("single_block_optimized", |b| {
        b.iter(|| {
            rt.block_on(async {
                let block = palantiri_minimal
                    .get_block_by_number(TEST_BLOCKS[0], false)
                    .await
                    .unwrap();
                black_box(block);
            })
        });
    });
    
    group.finish();
}

criterion_group!(benches, benchmark_http2_optimizations, benchmark_single_request_optimization);
criterion_main!(benches);