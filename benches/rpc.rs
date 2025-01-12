use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mordor::SlotSynchronizer;
use palantir::{
    palantiri::{
        node::{ChainEvent, Node},
        rpc::RpcClient,
        transport::http::TransportBuilder,
    },
    shire::concensus::{ConsensusConfig, ConsensusImpl},
    types::BlockHeader,
};
use std::{sync::Arc, time::Duration};
use tokio::runtime::Runtime;

// Import your node components
use alloy::primitives::B256;

// pub fn benchmark_sync_blocks(c: &mut Criterion) {
//     let rt = Runtime::new().unwrap();

//     // Setup RPC client and node
//     let rpc = RpcClient::new(
//         TransportBuilder::new(
//             "https://eth-mainnet.g.alchemy.com/v2/4yEoD1kdx0Eocdx_HFeGAOPsbysH3yRM".to_string()
//         ).build_http(),
//     );

//     let node = Node::new(
//         Arc::new(ConsensusImpl::new(
//             ConsensusConfig {
//                 chain_id: 1,
//                 finalized_block_number: 0,
//                 genesis_hash: B256::default(),
//                 finalized_block_hash: B256::default(),
//                 sync_period: 10,
//                 min_sync_comitee: 30,
//             },
//             Arc::new(rpc.clone()),
//         )),
//         Arc::new(rpc),
//     );

//     let mut group = c.benchmark_group("sync_operations");

//     // Benchmark block range sync with different sizes
//     let start_block = 17000000;
//     for size in [10].iter() {
//         group.bench_function(format!("sync_{}_blocks", size), |b| {
//             b.iter(|| {
//                 rt.block_on(async {
//                     black_box(
//                         node.sync_block_range(start_block, start_block + size).await.unwrap()
//                     )
//                 })
//             })
//         });
//     }

//     group.finish();
// }

pub fn benchmark_block_watching(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let rpc = RpcClient::new(
        TransportBuilder::new(
            "https://mainnet.infura.io/v3/2DCsBRUv8lDFmznC1BGik1pFKAL".to_string(),
        )
        .build_http(),
    );

    let node = Arc::new(Node::new(
        Arc::new(ConsensusImpl::new(
            ConsensusConfig {
                chain_id: 1,
                finalized_block_number: 0,
                genesis_hash: B256::default(),
                finalized_block_hash: B256::default(),
                sync_period: 10,
                min_sync_comitee: 30,
            },
            Arc::new(rpc.clone()),
        )),
        Arc::new(rpc),
    ));

    let mut group = c.benchmark_group("block_sync");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(50)); 

    group.bench_function("sync_10_blocks", |b| {
        b.iter(|| {
            rt.block_on(async {
                
                node.sync_block_range(100_000, 101_000).await.unwrap()
            })
        });
    });

    group.finish();
}

// pub fn benchmark_state_operations(c: &mut Criterion) {
//     let rt = Runtime::new().unwrap();

//     let rpc = RpcClient::new(
//         TransportBuilder::new(
//             "https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY".to_string()
//         ).build_http(),
//     );

//     let node = Node::new(
//         Arc::new(ConsensusImpl::new(
//             ConsensusConfig {
//                 chain_id: 1,
//                 finalized_block_number: 0,
//                 genesis_hash: B256::default(),
//                 finalized_block_hash: B256::default(),
//                 sync_period: 10,
//                 min_sync_comitee: 30,
//             },
//             Arc::new(rpc.clone()),
//         )),
//         Arc::new(rpc),
//     );

//     let mut group = c.benchmark_group("state_operations");

//     // Benchmark state read/write operations
//     group.bench_function("state_updates", |b| {
//         b.iter(|| {
//             rt.block_on(async {
//                 let mut state = node.SyncedState.as_ref().unwrap().write().await;
//                 black_box(state.current_block += 1);
//             })
//         });
//     });

//     group.bench_function("event_broadcast", |b| {
//         b.iter(|| {
//             rt.block_on(async {
//                 black_box(
//                     node.event_tx.send(ChainEvent::NewBlock(BlockHeader::default())).unwrap()
//                 )
//             })
//         });
//     });

//     group.finish();
// }

criterion_group!(
    benches,
    // benchmark_sync_blocks,
    benchmark_block_watching,
    // benchmark_state_operations
);
criterion_main!(benches);
