use std::time::Duration;

use ::palantiri::{hyper_rpc::RpcClient as HyperRpcClient, transport::http::TransportBuilder};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;

const RECENT_BLOCKS: [u64; 5] = [23218929, 23218928, 23218927, 23218926, 23218925];
const RPC_URL: &str = "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4";

pub fn benchmark_batching(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let rpc = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_http_hyper());

    let mut group = c.benchmark_group("batching_optimization");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(120));

    // Test 1: Individual requests (old way)
    group.bench_function("individual_requests", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut blocks = Vec::new();
                for &block_num in &RECENT_BLOCKS {
                    match rpc.get_block_by_number(block_num, true).await {
                        Ok(block) => blocks.push(block),
                        Err(_) => blocks.push(None),
                    }
                }
                black_box(blocks)
            })
        });
    });

    // Test 2: Batch request (new way)
    group.bench_function("batch_request", |b| {
        b.iter(|| {
            rt.block_on(async {
                let blocks = rpc.get_blocks_by_numbers(RECENT_BLOCKS.to_vec(), true).await;
                black_box(blocks)
            })
        });
    });

    group.finish();
}

pub fn benchmark_caching(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let rpc = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_http_hyper());

    let mut group = c.benchmark_group("caching_optimization");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));

    // Pre-warm cache
    rt.block_on(async {
        let _ = rpc.get_block_by_number(RECENT_BLOCKS[0], true).await;
    });

    // Test cached requests
    group.bench_function("cached_request", |b| {
        b.iter(|| {
            rt.block_on(async {
                let block = rpc.get_block_by_number(RECENT_BLOCKS[0], true).await;
                black_box(block)
            })
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_batching, benchmark_caching,);
criterion_main!(benches);
