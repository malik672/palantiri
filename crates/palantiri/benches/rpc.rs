use ::palantiri::{
    node::{ChainEvent, Node},
    rpc::RpcClient,
    transport::http::TransportBuilder,
};
use alloy_primitives::{address, Address};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mordor::SlotSynchronizer;
use parser::{hex_to_b256, types::BlockHeader};

use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::runtime::Runtime;

use alloy::{hex, primitives::B256};

pub fn benchmark_sync_blocks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let rpc = RpcClient::new(
        TransportBuilder::new(
            "https://mainnet.infura.io/v3/2DCsBRUv8lDFmznC1BGik1pFKAL".to_string(),
        )
        .build_http(),
    );

    let node = Node::new(Arc::new(rpc));

    let mut group = c.benchmark_group("sync_operations");

    let start_block = 17000000;
    for size in [10000].iter() {
        group.bench_function(format!("sync_{}_blocks", size), |b| {
            b.iter(|| {
                rt.block_on(async {
                    black_box(
                        node.sync_block_range(start_block, start_block + size)
                            .await
                            .unwrap(),
                    )
                })
            })
        });
    }

    group.finish();
}

pub fn benchmark_block_watching(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let rpc = RpcClient::new(
        TransportBuilder::new(
            "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4".to_string(),
        )
        .build_http(),
    );

    let node = Arc::new(Node::new(Arc::new(rpc)));

    let mut group = c.benchmark_group("block_sync");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(50));

    group.bench_function("sync_10_blocks", |b| {
        b.iter(|| rt.block_on(async { node.sync_block_range(100_000, 110_000).await.unwrap() }));
    });

    group.finish();
}

pub fn benchmark_get_logs(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let rpc = RpcClient::new(
        TransportBuilder::new(
            "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4".to_string(),
        )
        .build_http(),
    );

    let node = Arc::new(Node::new(Arc::new(rpc)));

    let mut group = c.benchmark_group("log_fetch");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(50));

    // USDC contract address for testing
    let address = Some(address!("1F98431c8aD98523631AE4a59f267346ea31F984"));

    group.bench_function("get_2000_logs", |b| {
        b.iter(|| {
            rt.block_on(async {
                let s = node
                    .rpc
                    .get_logs(20_000_000, 20_001_000, address, None)
                    .await
                    .unwrap();
                s
            })
        });
    });

    group.finish();
}

pub fn benchmark_get_tx_numbers(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let rpc = RpcClient::new(
        TransportBuilder::new(
            "https://mainnet.infura.io/v3/f5fa2813a91241dbb0decd8872ee2154".to_string(),
        )
        .build_http(),
    );

    let node = Arc::new(Node::new(Arc::new(rpc)));

    let mut group = c.benchmark_group("log_fetch");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(50));

    group.bench_function("get_2000_tx", |b| {
        b.iter(|| {
            rt.block_on(async {
                let s = node
                    .rpc
                    .get_transaction_by_tx_hash(
                        B256::from_str(
                            "b79b64182236284ad6753e1b5f506e7e6989912c25887575f82d64f23f6bf267",
                        )
                        .expect("ddhoulfsdfds"),
                    )
                    .await
                    .unwrap();
                s
            })
        });
    });

    group.finish();
}

pub fn benchmark_get_numbers(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let rpc = RpcClient::new(
        TransportBuilder::new(
            "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4".to_string(),
        )
        .build_http(),
    );

    let node = Arc::new(Node::new(Arc::new(rpc)));

    let mut group = c.benchmark_group("number_fetch");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(50));

    // USDC contract address for testing
    let address = Some(address!("1F98431c8aD98523631AE4a59f267346ea31F984"));

    group.bench_function("get_2000_logs", |b| {
        b.iter(|| {
            rt.block_on(async {
                let s = node
                    .rpc
                    .get_logs(20_000_000, 20_002_000, address, None)
                    .await
                    .unwrap();
                s
            })
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_sync_blocks,
    // benchmark_get_logs,
    // benchmark_get_tx_numbers
);
criterion_main!(benches);
