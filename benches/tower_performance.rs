use std::time::Duration;

use ::palantiri::{hyper_rpc::RpcClient as HyperRpcClient, transport::http::TransportBuilder};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;

const RECENT_BLOCK: u64 = 21000000;
const RPC_URL: &str = "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4";

pub fn benchmark_tower_vs_hyper(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Setup clients
    let palantiri_hyper = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_http_hyper_minimal());
    let palantiri_tower = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_tower());
    let palantiri_tower_optimized = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_tower_optimized());

    let mut group = c.benchmark_group("tower_vs_hyper");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(45));

    group.bench_function("hyper_minimal", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = palantiri_hyper.get_block_by_number(RECENT_BLOCK, false).await;
                black_box(result)
            })
        });
    });

    group.bench_function("tower_standard", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = palantiri_tower.get_block_by_number(RECENT_BLOCK, false).await;
                black_box(result)
            })
        });
    });

    group.bench_function("tower_optimized", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = palantiri_tower_optimized.get_block_by_number(RECENT_BLOCK, false).await;
                black_box(result)
            })
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_tower_vs_hyper);
criterion_main!(benches);