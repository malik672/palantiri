#[allow(unused_imports)]
use ::palantiri::{ transport::http::TransportBuilder, hyper_rpc::RpcClient as HyperRpcClient};
use alloy::{
    eips::BlockNumberOrTag,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;

use std::{sync::{atomic::{AtomicU64, Ordering}}};

use alloy::providers::{Provider, ProviderBuilder};

pub fn benchmark_number(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let rpc_url = "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4"
        .parse()
        .unwrap();
    let provider = ProviderBuilder::new().on_http(rpc_url);
    
    // Use atomic counter to avoid shared mutable state issues
    let block_counter = AtomicU64::new(22812202);
    
    let mut group = c.benchmark_group("ethereum_rpc");
    group.sample_size(50);
    group.measurement_time(std::time::Duration::from_secs(30));
    
    group.bench_function("get_numbers", |b| {
        b.iter(|| {
            rt.block_on(async {
                let block_num = block_counter.fetch_sub(1, Ordering::SeqCst);
                let result = provider
                    .get_block_by_number(BlockNumberOrTag::Number(block_num))
                    .await;
                
                // Handle errors gracefully for benchmarking
                match result {
                    Ok(block) => black_box(block),
                    Err(e) => {
                        eprintln!("Error fetching block {}: {}", block_num, e);
                        black_box(None)
                    }
                }
            })
        });
    });
    
    group.finish();
}

pub fn benchmark_get_block_numbers(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let rpc = HyperRpcClient::new(
        TransportBuilder::new(
            "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4",
        )
        .build_http_hyper(),
    );
    
    let block_counter = AtomicU64::new(22812202);
    
    let mut group = c.benchmark_group("ethereum_rpc");
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(30));
    
    group.bench_function("get_numbers_palantiri", |b| {
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

criterion_group!(
    benches,
    // benchmark_number,
    benchmark_get_block_numbers,
);
criterion_main!(benches);
