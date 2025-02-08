use alloy_primitives::{B256, U64};

use crate::{find_field, hex_to_b256, hex_to_u64};

#[derive(Debug, Default)]
pub struct FinalityCheckpoint {
    pub execution_optimistic: bool,
    pub finalized: bool,
    pub aggregation_bits: U64,
    pub signature: B256,
    pub data: CheckpointData,
    pub code: Option<u16>,
}

#[derive(Debug, Default)]
pub struct Source {
    pub epoch: U64,
    pub root: B256,
}

#[derive(Debug, Default)]
pub struct CheckpointData {
    pub slot: U64,
    pub index: U64,
    pub beacon_block_root: B256,
    pub source: Source,
    pub target: Source,
}

impl Source {
    pub fn parse_source(input: &[u8]) -> Option<Self> {
        let source = find_field(input, b"\"source\": {", b"}")?;
        let epoch = find_field(&input[source.0..source.1], b"\"epoch\":\"", b"\"")?;
        let root = find_field(&input[source.0..source.1], b"\"root\":\"", b"\"")?;

        Some(Self {
            epoch: hex_to_u64(&input[epoch.0..epoch.1]),
            root: hex_to_b256(&input[root.0..root.1]),
        })
    }

    pub fn parse_target(input: &[u8]) -> Option<Self> {
        let target = find_field(input, b"\"target\": {", b"}")?;
        let epoch = find_field(&input[target.0..target.1], b"\"epoch\":\"", b"\"")?;
        let root = find_field(&input[target.0..target.1], b"\"root\":\"", b"\"")?;

        Some(Self {
            epoch: hex_to_u64(&input[epoch.0..epoch.1]),
            root: hex_to_b256(&input[root.0..root.1]),
        })
    }
}

impl FinalityCheckpoint {
    pub fn parse(input: &[u8]) -> Option<Self> {
        if memchr::memmem::find(input, b"\"code\":").is_some() {
            let code = find_field(input, b"\"code\":", b",")?;
            let code_str = std::str::from_utf8(&input[code.0..code.1]).ok()?;
            return Some(Self {
                code: Some(code_str.parse().ok()?),
                ..Default::default()
            });
        }
        let optimistic = find_field(input, b"\"execution_optimistic\":", b",")?;
        let finalized = find_field(input, b"\"finalized\":", b",")?;
        let bits = find_field(input, b"\"aggregation_bits\":\"", b"\"")?;
        let sig = find_field(input, b"\"signature\":\"", b"\"")?;
        let data = CheckpointData::parse(input)?;

        Some(Self {
            execution_optimistic: input[optimistic.0..optimistic.1] == *b"true",
            finalized: input[finalized.0..finalized.1] == *b"true",
            aggregation_bits: hex_to_u64(&input[bits.0..bits.1]),
            signature: hex_to_b256(&input[sig.0..sig.1]),
            data,
            code: None,
        })
    }
}

impl CheckpointData {
    pub fn parse(input: &[u8]) -> Option<Self> {
        let slot = find_field(input, b"\"slot\":\"", b"\"")?;
        let index = find_field(input, b"\"index\":\"", b"\"")?;
        let root = find_field(input, b"\"beacon_block_root\":\"", b"\"")?;
        let source = Source::parse_source(&input)?;
        let target = Source::parse_target(&input)?;

        Some(Self {
            slot: hex_to_u64(&input[slot.0..slot.1]),
            index: hex_to_u64(&input[index.0..index.1]),
            beacon_block_root: hex_to_b256(&input[root.0..root.1]),
            source,
            target,
        })
    }
}
