#[allow(unused_imports)]
use ::palantiri::{rpc::RpcClient, transport::http::TransportBuilder, hyper_rpc::RpcClient as HyperRpcClient};
use alloy::{
    eips::BlockNumberOrTag,
    primitives::{address, U256},
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use palantiri::{hyper_rpc::RpcRequest, parser::types::TransactionRequest};
use tokio::runtime::Runtime;

use std::{sync::Arc, time::Duration};

use alloy::providers::{Provider, ProviderBuilder};

pub fn benchmark_get_logs(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let rpc = RpcClient::new(
        TransportBuilder::new(
            "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4".to_string(),
        )
        .build_http(),
    );

    let node = Arc::new(rpc);

    let mut group = c.benchmark_group("log_fetch");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(50));

    // USDC contract address for testing
    let address = Some(address!("1F98431c8aD98523631AE4a59f267346ea31F984"));

    group.bench_function("get_2000_logs", |b| {
        b.iter(|| {
            rt.block_on(async {
                let s = node
                    .get_logs(20_000_000, 20_001_000, address, None)
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

    let node = Arc::new(rpc);

    let mut group = c.benchmark_group("number_fetch");
    group.sample_size(10);

    group.bench_function("get_numbers", |b| {
        b.iter(|| {
            rt.block_on(async {
                let s = node.get_block_number().await;
                s
            })
        });
    });

    group.finish();
}

pub fn benchmark_estimate_gas(c: &mut Criterion) {
    let tx: TransactionRequest = TransactionRequest {
        from: Some(address!("8f54C8c2df62c94772ac14CcFc85603742976312")),
        to: Some(address!("44aa93095d6749a706051658b970b941c72c1d53")),
        gas: None,
        gas_price: Some(U256::from(26112348709 as u64)),
        value: None,
        data: Some("0xdd9c5f960000000000000000000000000d500b1d8e8ef31e21c99d1db9a6444d3adf12700000000000000000000000000000000000000000000000056bc75e2d631000000000000000000000000000000b3f868e0be5597d5db7feb59e1cadbb0fdda50a000000000000000000000000000000000000000000000001e1291b1bf0494000000000000000000000000000000000000000000000000001de460b131125fe970000000000000000000000008f54c8c2df62c94772ac14ccfc856037429763120000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000e0020d500B1d8E8eF31E21C99d1Db9A6444d3ADf12700215550133C4F0043E2e988b3c2e9C77e2C670eFe709Bfe30185CD07Ea01423b1E937929B44E4Ad8c40BbB5E7100ffff0186f1d8390222A3691C28938eC7404A1661E618e00185CD07Ea01423b1E937929B44E4Ad8c40BbB5E7100017ceB23fD6bC0adD59E62ac25578270cFf1b9f619026aaa010312692E9cADD3dDaaCE2E112A4e36397bd2f18a0085CD07Ea01423b1E937929B44E4Ad8c40BbB5E7100ffff01Ff5713FdbAD797b81539b5F9766859d4E050a6CC0085CD07Ea01423b1E937929B44E4Ad8c40BbB5E7100".to_string()),
        nonce: None,
    };

    let rt = Runtime::new().unwrap();

    let rpc = RpcClient::new(
        TransportBuilder::new(
            "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4".to_string(),
        )
        .build_http(),
    );

    let node = Arc::new(rpc);

    let mut group = c.benchmark_group("estimate_gas");
    group.sample_size(100);

    group.bench_function("get_estimate_gas", |b| {
        b.iter(|| {
            rt.block_on(async {
                let s = node.estimate_gas(&tx, None).await;
                s
            })
        });
    });

    group.finish();
}

pub fn benchmark_number(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let rpc_url = "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4"
        .parse()
        .unwrap();
    let provider = ProviderBuilder::new().on_http(rpc_url);

    let mut group = c.benchmark_group("number_fetch");
    group.sample_size(100);
    group.measurement_time(std::time::Duration::from_secs(44));
    let mut no = 22423460;

    group.bench_function("get_numbers", |b| {
        b.iter(|| {
            rt.block_on(async {
                let s =
                    black_box(provider.get_block_by_number(BlockNumberOrTag::Number(no))).await;
                    no = no - 1;
                black_box(s.unwrap())
            })
        });
    });
}

pub fn benchmark_get_block_numbers(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let rpc = HyperRpcClient::new(
        TransportBuilder::new(
            "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4".to_string(),
        )
        .build_http_hyper(),
    );

    let mut group = c.benchmark_group("number_fetch");
    group.sample_size(100);
    let mut no = 22423460;
    group.measurement_time(std::time::Duration::from_secs(44));

    group.bench_function("get_numbers_palantiri", |b| {
        b.iter(|| {
         
            rt.block_on(async {
                let s = black_box(rpc.get_block_by_number(no, true)).await;
                no = no - 1;
                black_box(s.unwrap())
            })
       
        });

        
    });

    group.finish();
}

pub fn benchmark_execute_raw(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let rpc = HyperRpcClient::new(
        TransportBuilder::new(
            "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4".to_string(),
        )
        .build_http_hyper(),
    );

    let mut group = c.benchmark_group("number_fetch");
    group.sample_size(100);
    let mut no = 22440939;
    group.measurement_time(std::time::Duration::from_secs(44));

    group.bench_function("get_execute_raw", |b| {
        b.iter(|| {
         
            rt.block_on(async {
                let request = RpcRequest {
                    jsonrpc: "2.0",
                    method: "eth_getBlockByNumber",
                    params: serde_json::json!([format!("0x{:x}", no), true]),
                    id: 1,
                };
        
                let s = black_box(rpc.execute_raw(request)).await;
                no = no - 1;
                black_box(s.unwrap())
            })
       
        });

        
    });

    group.finish();
}

criterion_group!(
    benches,
    // benchmark_number,
    benchmark_get_block_numbers,
    // benchmark_execute_raw,
);
criterion_main!(benches);
