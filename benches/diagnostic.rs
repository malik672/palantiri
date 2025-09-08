use std::time::{Duration, Instant};

use ::palantiri::{hyper_rpc::RpcClient as HyperRpcClient, transport::http::TransportBuilder};
use alloy::{
    eips::BlockNumberOrTag,
    providers::{Provider, ProviderBuilder},
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;

const RECENT_BLOCK: u64 = 23218929;
const OLD_BLOCK: u64 = 22812202;
const RPC_URL: &str = "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4";

pub fn diagnostic_network_vs_parsing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Setup clients
    let rpc_url = RPC_URL.parse().unwrap();
    let alloy_provider = ProviderBuilder::new().on_http(rpc_url);

    let palantiri_rpc = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_http_hyper());

    let mut group = c.benchmark_group("network_vs_parsing");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));

    // Test 1: Network timing only for Palantiri (raw response)
    group.bench_function("palantiri_network_only_recent", |b| {
        b.iter(|| {
            rt.block_on(async {
                let request = palantiri::hyper_rpc::RpcRequest {
                    jsonrpc: "2.0",
                    method: "eth_getBlockByNumber",
                    params: serde_json::json!([format!("0x{:x}", RECENT_BLOCK), true]),
                    id: 1,
                };

                let start = Instant::now();
                let raw_response = palantiri_rpc.execute_raw(request).await;
                let network_time = start.elapsed();

                black_box((network_time, raw_response.map(|r| r.len())))
            })
        });
    });

    // Test 2: Full parsing time for Palantiri
    group.bench_function("palantiri_full_recent", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = palantiri_rpc.get_block_by_number(RECENT_BLOCK, true).await;
                black_box(result)
            })
        });
    });

    // Test 3: Compare with old block network timing
    group.bench_function("palantiri_network_only_old", |b| {
        b.iter(|| {
            rt.block_on(async {
                let request = palantiri::hyper_rpc::RpcRequest {
                    jsonrpc: "2.0",
                    method: "eth_getBlockByNumber",
                    params: serde_json::json!([format!("0x{:x}", OLD_BLOCK), true]),
                    id: 1,
                };

                let start = Instant::now();
                let raw_response = palantiri_rpc.execute_raw(request).await;
                let network_time = start.elapsed();

                black_box((network_time, raw_response.map(|r| r.len())))
            })
        });
    });

    // Test 4: Compare Alloy network timing
    group.bench_function("alloy_recent", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = alloy_provider
                    .get_block_by_number(BlockNumberOrTag::Number(RECENT_BLOCK))
                    .await;
                black_box(result)
            })
        });
    });

    group.finish();
}

pub fn diagnostic_parsing_only(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let palantiri_rpc = HyperRpcClient::new(TransportBuilder::new(RPC_URL).build_http_hyper());

    // Pre-fetch some data to test parsing in isolation
    let recent_block_data = rt.block_on(async {
        let request = palantiri::hyper_rpc::RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getBlockByNumber",
            params: serde_json::json!([format!("0x{:x}", RECENT_BLOCK), true]),
            id: 1,
        };
        palantiri_rpc.execute_raw(request).await.unwrap_or_default()
    });

    let old_block_data = rt.block_on(async {
        let request = palantiri::hyper_rpc::RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getBlockByNumber",
            params: serde_json::json!([format!("0x{:x}", OLD_BLOCK), true]),
            id: 1,
        };
        palantiri_rpc.execute_raw(request).await.unwrap_or_default()
    });

    let mut group = c.benchmark_group("parsing_only");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("parse_recent_block", |b| {
        b.iter(|| {
            let parsed = palantiri::parser::block_parser::parse_block(&recent_block_data);
            black_box(parsed)
        });
    });

    group.bench_function("parse_old_block", |b| {
        b.iter(|| {
            let parsed = palantiri::parser::block_parser::parse_block(&old_block_data);
            black_box(parsed)
        });
    });

    // Also test raw data sizes
    group.bench_function("data_size_comparison", |b| {
        b.iter(|| black_box((recent_block_data.len(), old_block_data.len())));
    });

    group.finish();
}

criterion_group!(benches, diagnostic_network_vs_parsing, diagnostic_parsing_only,);
criterion_main!(benches);
