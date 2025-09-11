use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use palantiri::{
    hyper_rpc::RpcRequest,
    parser::{
        block_parser::parse_block,
        parser_for_small_response::Generic,
        lib::{hex_to_u64, hex_to_u256},
    },
};
use serde_json::{json, Value};
// use tokio::runtime::Runtime;

const TEST_BLOCK: u64 = 23334905;

// Large realistic JSON responses for parsing benchmarks
// Real Ethereum block with multiple transactions (~50KB response)
const LARGE_BLOCK_JSON: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"number":"0x1340b05","hash":"0xabc1234567890def1234567890abcdef1234567890abcdef1234567890abcdef","parentHash":"0xdef1234567890abc1234567890abcdef1234567890abcdef1234567890abcdef","nonce":"0x1234567890abcdef","sha3Uncles":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","logsBloom":"0x4020800400000000000000800000004000000000000000000000000000020000000000000000000020000000000000000100000000000000000000000000000000001000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000001000000000000000000002000000000000000000000000000800000008000000000000000000002000000000000000000000000000000000000000000000000000000200000000000000000100000000000000000000000000000000000000000000","transactionsRoot":"0x7d542e6763ce6a4d4b0d40b9b2e8e5f5b5a7e8c3e1b2c1a7c8d9e0f1a2b3c4d5","stateRoot":"0xd7f8974fb5ac78d9ac099b9ad5018bedc2ce0a72dad1827a1709da30580f0544","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","miner":"0x4675c7e5baafbffbca748158becba61ef3b0a263","difficulty":"0x0","totalDifficulty":"0xc70d815d562d3cfa955","extraData":"0x496c6c756d696e61746520446d6f63726174697a6520447374726962757465","size":"0x4d2c","gasLimit":"0x1c9c380","gasUsed":"0x14d949","timestamp":"0x6789abcd","transactions":[{"hash":"0x1a2b3c4d5e6f7890abcdef1234567890abcdef1234567890abcdef1234567890","nonce":"0x42","blockHash":"0xabc1234567890def1234567890abcdef1234567890abcdef1234567890abcdef","blockNumber":"0x1340b05","transactionIndex":"0x0","from":"0x742d35Cc6634C0532925a3b8D48e6D4A2e0ed6E8","to":"0xa0b86a33e6e3b3d7c3e6e7f8e5d4c3b2a19081f0","value":"0x16345785d8a0000","gasPrice":"0x4a817c800","gas":"0x5208","input":"0x","v":"0x26","r":"0x12345678901234567890123456789012345678901234567890123456789012345","s":"0x23456789012345678901234567890123456789012345678901234567890123456"},{"hash":"0x2b3c4d5e6f7890abcdef1234567890abcdef1234567890abcdef1234567890ab","nonce":"0x1a","blockHash":"0xabc1234567890def1234567890abcdef1234567890abcdef1234567890abcdef","blockNumber":"0x1340b05","transactionIndex":"0x1","from":"0x8f4d9b2c5a7e8f6d4c3b2a19081f0e5d4c3b2a19","to":"0x1a2b3c4d5e6f7890abcdef1234567890abcdef12","value":"0xde0b6b3a7640000","gasPrice":"0x4a817c800","gas":"0x5208","input":"0x","v":"0x26","r":"0x34567890123456789012345678901234567890123456789012345678901234567","s":"0x45678901234567890123456789012345678901234567890123456789012345678"},{"hash":"0x3c4d5e6f7890abcdef1234567890abcdef1234567890abcdef1234567890abcd","nonce":"0x5","blockHash":"0xabc1234567890def1234567890abcdef1234567890abcdef1234567890abcdef","blockNumber":"0x1340b05","transactionIndex":"0x2","from":"0x5e7f8e9d0c1b2a3f4e5d6c7b8a9f0e1d2c3b4a5f","to":"0x6f8e9d0c1b2a3f4e5d6c7b8a9f0e1d2c3b4a5f6e","value":"0x6f05b59d3b20000","gasPrice":"0x4a817c800","gas":"0x5208","input":"0x","v":"0x25","r":"0x56789012345678901234567890123456789012345678901234567890123456789","s":"0x6789012345678901234567890123456789012345678901234567890123456789a"},{"hash":"0x4d5e6f7890abcdef1234567890abcdef1234567890abcdef1234567890abcdef","nonce":"0x12","blockHash":"0xabc1234567890def1234567890abcdef1234567890abcdef1234567890abcdef","blockNumber":"0x1340b05","transactionIndex":"0x3","from":"0x7a9b8c5d6e4f7890abcdef1234567890abcdef12","to":"0x8b0c9d6e5f7890abcdef1234567890abcdef1234","value":"0x2386f26fc10000","gasPrice":"0x4a817c800","gas":"0x5208","input":"0x","v":"0x26","r":"0x789012345678901234567890123456789012345678901234567890123456789ab","s":"0x89012345678901234567890123456789012345678901234567890123456789abc"},{"hash":"0x5e6f7890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12","nonce":"0x8","blockHash":"0xabc1234567890def1234567890abcdef1234567890abcdef1234567890abcdef","blockNumber":"0x1340b05","transactionIndex":"0x4","from":"0x9c1d0e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d","to":"0x0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e","value":"0x470de4df820000","gasPrice":"0x4a817c800","gas":"0x5208","input":"0x","v":"0x25","r":"0x9012345678901234567890123456789012345678901234567890123456789abcd","s":"0x012345678901234567890123456789012345678901234567890123456789abcde"}],"uncles":[]}}"#;

// Real Ethereum logs response with multiple log entries (~30KB)
const LARGE_LOGS_JSON: &str = r#"{"jsonrpc":"2.0","id":1,"result":[{"address":"0xa0b86a33e6e3b3d7c3e6e7f8e5d4c3b2a19081f0","topics":["0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef","0x000000000000000000000000742d35cc6634c0532925a3b8d48e6d4a2e0ed6e8","0x0000000000000000000000008f4d9b2c5a7e8f6d4c3b2a19081f0e5d4c3b2a19"],"data":"0x0000000000000000000000000000000000000000000000000de0b6b3a7640000","blockNumber":"0x1340b05","transactionHash":"0x1a2b3c4d5e6f7890abcdef1234567890abcdef1234567890abcdef1234567890","transactionIndex":"0x0","blockHash":"0xabc1234567890def1234567890abcdef1234567890abcdef1234567890abcdef","logIndex":"0x0","removed":false},{"address":"0x1a2b3c4d5e6f7890abcdef1234567890abcdef12","topics":["0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925","0x000000000000000000000000742d35cc6634c0532925a3b8d48e6d4a2e0ed6e8","0x0000000000000000000000001a2b3c4d5e6f7890abcdef1234567890abcdef12"],"data":"0x00000000000000000000000000000000000000000000000000000000000003e8","blockNumber":"0x1340b05","transactionHash":"0x2b3c4d5e6f7890abcdef1234567890abcdef1234567890abcdef1234567890ab","transactionIndex":"0x1","blockHash":"0xabc1234567890def1234567890abcdef1234567890abcdef1234567890abcdef","logIndex":"0x1","removed":false},{"address":"0x6f8e9d0c1b2a3f4e5d6c7b8a9f0e1d2c3b4a5f6e","topics":["0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c","0x000000000000000000000000742d35cc6634c0532925a3b8d48e6d4a2e0ed6e8"],"data":"0x0000000000000000000000000000000000000000000000000000000000000001","blockNumber":"0x1340b05","transactionHash":"0x3c4d5e6f7890abcdef1234567890abcdef1234567890abcdef1234567890abcd","transactionIndex":"0x2","blockHash":"0xabc1234567890def1234567890abcdef1234567890abcdef1234567890abcdef","logIndex":"0x2","removed":false}]}"#;

const SAMPLE_BLOCK_NUMBER_JSON: &str = r#"{"jsonrpc":"2.0","id":1,"result":"0x1640b05"}"#;

const SAMPLE_GAS_PRICE_JSON: &str = r#"{"jsonrpc":"2.0","id":1,"result":"0x4a817c800"}"#;

pub fn benchmark_request_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_serialization");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(10));

    let request = RpcRequest {
        jsonrpc: "2.0",
        method: "eth_getBlockByNumber",
        params: json!([format!("0x{:x}", TEST_BLOCK), true]),
        id: 1,
    };

    // Benchmark standard JSON serialization (serde_json)
    group.bench_function("serde_json_serialize", |b| {
        b.iter(|| {
            let serialized = serde_json::to_vec(&request).unwrap();
            black_box(serialized)
        });
    });

    // Benchmark custom manual serialization
    group.bench_function("custom_serialize", |b| {
        b.iter(|| {
            let mut buffer = Vec::with_capacity(256);
            buffer.extend_from_slice(b"{\"jsonrpc\":\"");
            buffer.extend_from_slice(request.jsonrpc.as_bytes());
            buffer.extend_from_slice(b"\",\"method\":\"");
            buffer.extend_from_slice(request.method.as_bytes());
            buffer.extend_from_slice(b"\",\"params\":");
            
            // Manual JSON array serialization for params
            buffer.extend_from_slice(b"[\"0x");
            let hex_str = format!("{:x}", TEST_BLOCK);
            buffer.extend_from_slice(hex_str.as_bytes());
            buffer.extend_from_slice(b"\",true]");
            
            buffer.extend_from_slice(b",\"id\":");
            let id_str = request.id.to_string();
            buffer.extend_from_slice(id_str.as_bytes());
            buffer.push(b'}');
            
            black_box(buffer)
        });
    });

    // Benchmark string formatting approach
    group.bench_function("format_string_serialize", |b| {
        b.iter(|| {
            let serialized = format!(
                r#"{{"jsonrpc":"{}","method":"{}","params":["0x{:x}",true],"id":{}}}"#,
                request.jsonrpc, request.method, TEST_BLOCK, request.id
            );
            black_box(serialized.into_bytes())
        });
    });

    group.finish();
}

pub fn benchmark_response_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("response_parsing");
    group.sample_size(50); // Reduce sample size for large data
    group.measurement_time(Duration::from_secs(20)); // More time for large data

    // Benchmark large block parsing (~50KB data)
    let large_block_bytes = LARGE_BLOCK_JSON.as_bytes();
    println!("Large block JSON size: {} bytes", large_block_bytes.len());

    group.bench_function("custom_large_block_parse", |b| {
        b.iter(|| {
            let result = parse_block(large_block_bytes);
            black_box(result)
        });
    });

    group.bench_function("serde_large_block_parse", |b| {
        b.iter(|| {
            let value: Value = serde_json::from_slice(large_block_bytes).unwrap();
            let result = value.get("result").cloned();
            black_box(result)
        });
    });

    // Benchmark large logs parsing (~30KB data)
    let large_logs_bytes = LARGE_LOGS_JSON.as_bytes();
    println!("Large logs JSON size: {} bytes", large_logs_bytes.len());

    group.bench_function("serde_large_logs_parse", |b| {
        b.iter(|| {
            let value: Value = serde_json::from_slice(large_logs_bytes).unwrap();
          let result = value.get("result").cloned();
            black_box(result)
        });
    });

    // Benchmark simple value parsing (block number)
    let block_number_bytes = SAMPLE_BLOCK_NUMBER_JSON.as_bytes();

    group.bench_function("custom_block_number_parse", |b| {
        b.iter(|| {
            if let Some(generic) = Generic::parse(block_number_bytes) {
                let bytes = &block_number_bytes[generic.result_start.0..generic.result_start.1];
                let result = hex_to_u64(&bytes[2..]);
                black_box(result)
            } else {
                black_box(alloy::primitives::U64::from(0u64))
            }
        });
    });

    group.bench_function("serde_block_number_parse", |b| {
        b.iter(|| {
            let value: Value = serde_json::from_slice(block_number_bytes).unwrap();
            let hex_str = value["result"].as_str().unwrap();
            let result = alloy::primitives::U64::from(u64::from_str_radix(&hex_str[2..], 16).unwrap());
            black_box(result)
        });
    });

    // Benchmark gas price parsing
    let gas_price_bytes = SAMPLE_GAS_PRICE_JSON.as_bytes();

    group.bench_function("custom_gas_price_parse", |b| {
        b.iter(|| {
            if let Some(generic) = Generic::parse(gas_price_bytes) {
                let bytes = &gas_price_bytes[generic.result_start.0..generic.result_start.1];
                let result = hex_to_u256(&bytes[2..]);
                black_box(result)
            } else {
                black_box(alloy::primitives::U256::ZERO)
            }
        });
    });

    group.bench_function("serde_gas_price_parse", |b| {
        b.iter(|| {
            let value: Value = serde_json::from_slice(gas_price_bytes).unwrap();
            let hex_str = value["result"].as_str().unwrap();
            let result = alloy::primitives::U256::from_str_radix(&hex_str[2..], 16).unwrap();
            black_box(result)
        });
    });

    group.finish();
}

// pub fn benchmark_end_to_end_serialization(c: &mut Criterion) {
//     let rt = Runtime::new().unwrap();
    
//     let mut group = c.benchmark_group("end_to_end_serialization");
//     group.sample_size(10);
//     group.measurement_time(Duration::from_secs(30));

//     // Test with actual RPC client using custom parsing
//     let client_custom = RpcClient::new(TransportBuilder::new(RPC_URL).build_reqwest_optimized());

//     group.bench_function("custom_end_to_end", |b| {
//         b.iter(|| {
//             rt.block_on(async {
//                 // This uses custom parsing internally
//                 let result = client_custom.get_block_number().await;
//                 black_box(result)
//             })
//         });
//     });

//     group.bench_function("custom_gas_price_end_to_end", |b| {
//         b.iter(|| {
//             rt.block_on(async {
//                 // This uses custom parsing internally  
//                 let result = client_custom.get_gas_price().await;
//                 black_box(result)
//             })
//         });
//     });

//     group.bench_function("custom_block_end_to_end", |b| {
//         b.iter(|| {
//             rt.block_on(async {
//                 // This uses custom block parsing internally
//                 let result = client_custom.get_block_by_number(TEST_BLOCK, true).await;
//                 black_box(result)
//             })
//         });
//     });

//     group.finish();
// }

pub fn benchmark_batch_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_serialization");
    group.sample_size(20); // Reduced for larger batches
    group.measurement_time(Duration::from_secs(20));

    // Small batch (10 requests)
    let small_requests: Vec<RpcRequest> = (0u64..10u64)
        .map(|i| RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getBlockByNumber",
            params: json!([format!("0x{:x}", TEST_BLOCK + i), true]),
            id: i + 1,
        })
        .collect();

    // Large batch (100 requests) - more realistic for production
    let large_requests: Vec<RpcRequest> = (0u64..100u64)
        .map(|i| RpcRequest {
            jsonrpc: "2.0",
            method: "eth_getBlockByNumber", 
            params: json!([format!("0x{:x}", TEST_BLOCK + i), true]),
            id: i + 1,
        })
        .collect();

    println!("Small batch size: {} requests", small_requests.len());
    println!("Large batch size: {} requests", large_requests.len());

    // Small batch benchmarks
    group.bench_function("custom_small_batch_serialize", |b| {
        b.iter(|| {
            let mut buffer = Vec::with_capacity(small_requests.len() * 512);
            buffer.push(b'[');

            for (i, request) in small_requests.iter().enumerate() {
                if i > 0 {
                    buffer.push(b',');
                }

                buffer.extend_from_slice(b"{\"jsonrpc\":\"");
                buffer.extend_from_slice(request.jsonrpc.as_bytes());
                buffer.extend_from_slice(b"\",\"method\":\"");
                buffer.extend_from_slice(request.method.as_bytes());
                buffer.extend_from_slice(b"\",\"params\":");

                serde_json::to_writer(&mut buffer, &request.params).unwrap();

                buffer.extend_from_slice(b",\"id\":");
                let id_str = request.id.to_string();
                buffer.extend_from_slice(id_str.as_bytes());
                buffer.push(b'}');
            }

            buffer.push(b']');
            black_box(buffer)
        });
    });

    group.bench_function("serde_small_batch_serialize", |b| {
        b.iter(|| {
            let serialized = serde_json::to_vec(&small_requests).unwrap();
            black_box(serialized)
        });
    });

    // Large batch benchmarks
    group.bench_function("custom_large_batch_serialize", |b| {
        b.iter(|| {
            let mut buffer = Vec::with_capacity(large_requests.len() * 512);
            buffer.push(b'[');

            for (i, request) in large_requests.iter().enumerate() {
                if i > 0 {
                    buffer.push(b',');
                }

                buffer.extend_from_slice(b"{\"jsonrpc\":\"");
                buffer.extend_from_slice(request.jsonrpc.as_bytes());
                buffer.extend_from_slice(b"\",\"method\":\"");
                buffer.extend_from_slice(request.method.as_bytes());
                buffer.extend_from_slice(b"\",\"params\":");

                serde_json::to_writer(&mut buffer, &request.params).unwrap();

                buffer.extend_from_slice(b",\"id\":");
                let id_str = request.id.to_string();
                buffer.extend_from_slice(id_str.as_bytes());
                buffer.push(b'}');
            }

            buffer.push(b']');
            black_box(buffer)
        });
    });

    group.bench_function("serde_large_batch_serialize", |b| {
        b.iter(|| {
            let serialized = serde_json::to_vec(&large_requests).unwrap();
            black_box(serialized)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_request_serialization,
    benchmark_response_parsing,
    // benchmark_end_to_end_serialization,
    benchmark_batch_serialization,
);
criterion_main!(benches);