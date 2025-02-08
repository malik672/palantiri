use alloy_primitives::B256;

use crate::{find_field, hex_to_b256};

#[derive(Debug)]
pub struct BeaconStateRoot {
    pub root: B256,
    pub code: Option<u16>,
}

#[derive(Debug)]
struct RawJsonResponse<'a> {
    _code: Option<u16>,
    pub data: &'a [u8],
    pub data_start: usize,
    pub data_end: usize,
}

impl BeaconStateRoot {
    pub fn parse(input: &[u8]) -> Option<Self> {
        let root = find_field(input, b"\"root\":\"", b"\"")?;

        Some(Self {
            root: hex_to_b256(&input[root.0..root.1]),
            code: None,
        })
    }
}

impl<'a> RawJsonResponse<'a> {
    fn parse_beacon_state_root(input: &'a [u8]) -> Option<Self> {
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

pub fn parse_beacon_state_root(input: &[u8]) -> Option<BeaconStateRoot> {
    RawJsonResponse::parse_beacon_state_root(input)
        .and_then(|r| BeaconStateRoot::parse(&r.data[r.data_start..=r.data_end]))
}
