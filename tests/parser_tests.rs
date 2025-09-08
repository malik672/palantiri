use alloy::primitives::{B256, U256, U64};
use palantiri::parser::{
    block_parser::parse_block,
    lib::{hex_to_b256, hex_to_u256, hex_to_u64},
    log_parser::parse_logs,
    parser_for_small_response::{Generic, RawFee},
    tx_parser::parse_transaction,
};

fn b256_of(hex64: &str) -> B256 {
    // helper to construct a 0x-prefixed 32-byte hex into B256
    let mut s = String::from("0x");
    s.push_str(hex64);
    hex_to_b256(s.as_bytes())
}

#[test]
fn test_hex_helpers() {
    assert_eq!(hex_to_u64(b"0x0"), U64::from(0));
    assert_eq!(hex_to_u64(b"0x1a"), U64::from(0x1a));
    assert_eq!(hex_to_u64(b"1a"), U64::from(0x1a));

    let v = hex_to_u256(b"0x01");
    assert_eq!(v, U256::from(1u64));

    let h = b256_of(&"00".repeat(32));
    assert_eq!(h, B256::ZERO);
}

#[test]
fn test_parse_transaction_basic() {
    let json = r#"{
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "blockHash": "0x00000000000000000000000000000000000000000000000000000000000000aa",
            "blockNumber": "0x1",
            "hash": "0x00000000000000000000000000000000000000000000000000000000000000bb",
            "input": "0x",
            "r": "0x00000000000000000000000000000000000000000000000000000000000000cc",
            "s": "0x00000000000000000000000000000000000000000000000000000000000000dd",
            "v": "0x1",
            "gas": "0x5208",
            "gasPrice": "0x3b9aca00",
            "from": "0x1111111111111111111111111111111111111111",
            "transactionIndex": "0x0",
            "to": "0x2222222222222222222222222222222222222222",
            "value": "0x0",
            "nonce": "0x0"
        }
    }"#;

    let tx = parse_transaction(json.as_bytes()).expect("tx should parse");
    assert_eq!(tx.block_number.unwrap(), U64::from(1));
    assert_eq!(tx.gas, U256::from(0x5208u64));
    assert_eq!(tx.gas_price, U64::from(1_000_000_000));
}

#[test]
fn test_parse_logs_with_4_topics() {
    // Current parser expects exactly 4 topics; use 4 to validate path
    let json = r#"{
        "jsonrpc": "2.0",
        "id": 1,
        "result": [
            {
                "address": "0x1111111111111111111111111111111111111111",
                "topics": [
                    "0x0000000000000000000000000000000000000000000000000000000000000001",
                    "0x0000000000000000000000000000000000000000000000000000000000000002",
                    "0x0000000000000000000000000000000000000000000000000000000000000003",
                    "0x0000000000000000000000000000000000000000000000000000000000000004"
                ],
                "data": "0x",
                "blockNumber": "0x1",
                "blockHash": "0x00000000000000000000000000000000000000000000000000000000000000aa",
                "transactionHash": "0x00000000000000000000000000000000000000000000000000000000000000bb",
                "transactionIndex": "0x0",
                "logIndex": "0x0",
                "removed": false
            }
        ]
    }"#;

    let logs = parse_logs(json.as_bytes());
    assert_eq!(logs.len(), 1);
    let l = &logs[0];
    assert_eq!(l.topics.len(), 4);
    assert_eq!(l.block_number.unwrap(), U64::from(1));
}

#[test]
fn test_parse_block_smoke() {
    // Minimal but structurally complete block result
    let block_json = format!(
        "{{\n  \"jsonrpc\": \"2.0\",\n  \"id\": 1,\n  \"result\": {{\n    \"number\": \"0x10\",\n    \"hash\": \"0x{}\",\n    \"parentHash\": \"0x{}\",\n    \"sha3Uncles\": \"0x{}\",\n    \"miner\": \"0x1111111111111111111111111111111111111111\",\n    \"stateRoot\": \"0x{}\",\n    \"transactionsRoot\": \"0x{}\",\n    \"receiptsRoot\": \"0x{}\",\n    \"logsBloom\": \"0x{}\",\n    \"difficulty\": \"0x0\",\n    \"gasLimit\": \"0x5208\",\n    \"gasUsed\": \"0x0\",\n    \"timestamp\": \"0x5\",\n    \"extraData\": \"0x\",\n    \"mixHash\": \"0x{}\",\n    \"nonce\": \"0x0\",\n    \"baseFeePerGas\": \"0x0\",\n    \"transactions\": [\n      \"0x{}\",\n      \"0x{}\"\n    ],\n    \"uncles\": [\n      \"0x{}\"\n    ]\n  }}\n}}",
        "aa".repeat(32),
        "ab".repeat(32),
        "ac".repeat(32),
        "ad".repeat(32),
        "ae".repeat(32),
        "af".repeat(32),
        "00".repeat(256),
        "b0".repeat(32),
        "b1".repeat(32),
        "b2".repeat(32),
        "b3".repeat(32),
    );

    let block = parse_block(block_json.as_bytes()).expect("block should parse");
    // number parsing should work and transactions should be collected
    assert_eq!(block.number, U64::from(0x10));
    assert_eq!(block.transactions.len(), 2);
}

#[test]
fn test_parse_block_null_result() {
    let json = r#"{"jsonrpc":"2.0","id":1,"result":null}"#;
    let block = parse_block(json.as_bytes());
    assert!(block.is_none());
}

#[test]
fn test_parse_logs_empty() {
    let json = r#"{"jsonrpc":"2.0","id":1,"result":[]}"#;
    let logs = parse_logs(json.as_bytes());
    assert_eq!(logs.len(), 0);
}

#[test]
fn test_generic_small_result_parsing() {
    // Simulate a simple RPC like eth_chainId
    let json = br#"{"jsonrpc":"2.0","id":1,"result":"0x1"}"#;
    let g = Generic::parse(json).expect("generic should parse");
    let slice = &json[g.result_start.0..g.result_start.1];
    assert_eq!(hex_to_u64(slice), U64::from(1));
}

#[test]
fn test_raw_fee_parsing() {
    let json = br#"{
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "oldestBlock": "0x10",
            "reward": [
                ["0x1", "0x2"],
                ["0x3", "0x4"]
            ],
            "baseFeePerGas": ["0x5", "0x6"],
            "gasUsedRatio": ["0x0", "0x1"],
            "baseFeePerBlobGas": ["0x7", "0x8"]
        }
    }"#;

    let fee = RawFee::parse(json).expect("fee should parse");
    // Validate we captured slices for fields (not decoding them fully here)
    let oldest = &json[fee.oldest_block.0..fee.oldest_block.1];
    assert_eq!(oldest, b"0x10");
    assert!(fee.reward.len() >= 2);
    assert_eq!(&json[fee.base_fee_per_gas[0].0..fee.base_fee_per_gas[0].1], b"0x5");
}
