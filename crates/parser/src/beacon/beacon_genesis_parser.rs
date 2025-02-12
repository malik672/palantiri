use alloy_primitives::{B256, U64};

use crate::{find_field, hex_to_b256, hex_to_u64};

#[derive(Debug)]
pub struct BeaconGenesis {
    pub genesis_time: U64,
    pub genesis_validators_root: B256,
    pub genesis_fork_version: B256,
}

#[derive(Debug)]
struct RawJsonResponse<'a> {
    _code: Option<u16>,
    data: &'a [u8],
    data_start: usize,
    data_end: usize,
}

impl BeaconGenesis {
    pub fn parse(input: &[u8]) -> Option<Self> {
        let genesis_time = find_field(input, b"\"genesis_time\":\"", b"\"")?;
        let genesis_validators_root = find_field(input, b"\"genesis_validators_root\":\"", b"\"")?;
        let genesis_fork_version = find_field(input, b"\"genesis_fork_version\":\"", b"\"")?;

        Some(Self {
            genesis_time: hex_to_u64(&input[genesis_time.0..genesis_time.1]),
            genesis_validators_root: hex_to_b256(
                &input[genesis_validators_root.0..genesis_validators_root.1],
            ),
            genesis_fork_version: hex_to_b256(
                &input[genesis_fork_version.0..genesis_fork_version.1],
            ),
        })
    }
}

impl<'a> RawJsonResponse<'a> {
    pub fn parse_beacon_genesis(input: &'a [u8]) -> Option<Self> {
        let data_marker = b"\"data\":";
        let data_start = memchr::memmem::find(input, data_marker)?;
        if memchr::memmem::find(input, b"\"code\":").is_some() {
            let code = find_field(input, b"\"code\":", b",")?;
            let code_str = std::str::from_utf8(&input[code.0..code.1]).ok()?;
            return Some(Self {
                _code: Some(code_str.parse().ok()?),
                data: input,
                data_start: 0,
                data_end: 0,
            });
        }

        let pos = data_start + data_marker.len();
        let mut bracket_depth = 0;
        let mut data_end = pos;

        for (i, &byte) in input[pos..].iter().enumerate() {
            match byte {
                b'{' => bracket_depth += 1,
                b'}' => {
                    bracket_depth -= 1;
                    if bracket_depth == 0 {
                        data_end = pos + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }

        Some(Self {
            _code: None,
            data: input,
            data_start: pos,
            data_end,
        })
    }
}

pub fn parse_beacon_genesis(input: &[u8]) -> Option<BeaconGenesis> {
    RawJsonResponse::parse_beacon_genesis(input)
        .and_then(|r| BeaconGenesis::parse(&r.data[r.data_start..=r.data_end]))
}
