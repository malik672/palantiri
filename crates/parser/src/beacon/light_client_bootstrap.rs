use alloy_primitives::{B256, U64};

use crate::{find_field, hex_to_b256};

#[derive(Debug, Default)]
pub struct LightClientBootstrap<'a> {
    pub version: &'a str,
    pub header: Header,
    pub current_sync_committee: CurrentSyncCommittee,
    pub current_sync_committee_branch: Vec<B256>,
    pub code: Option<u16>,
}

#[derive(Debug, Default)]
pub struct Header {
    pub beacon: Beacon,
}

#[derive(Debug, Default)]
pub struct Beacon {
    pub slot: U64,
    pub proposer_index: U64,
    pub parent_root: B256,
    pub state_root: B256,
    pub body_root: B256,
}

#[derive(Debug, Default)]
pub struct CurrentSyncCommittee {
    pub pub_keys: Vec<B256>,
    pub aggregate_pubkey: B256,
}

impl<'a> LightClientBootstrap<'a> {
    pub fn parse(input: &'a [u8]) -> Option<Self> {
        if memchr::memmem::find(input, b"\"code\":").is_some() {
            let code = find_field(input, b"\"code\":", b",")?;
            let code_str = std::str::from_utf8(&input[code.0..code.1]).ok()?;
            return Some(Self {
                code: Some(code_str.parse().ok()?),
                ..Default::default()
            });
        }

        let version = find_field(input, b"\"version\":\"", b"\"")?;
        let slot = find_field(input, b"\"slot\":\"", b"\"")?;
        let proposer_index = find_field(input, b"\"proposer_index\":\"", b"\"")?;
        let parent_root = find_field(input, b"\"parent_root\":\"", b"\"")?;
        let state_root = find_field(input, b"\"state_root\":\"", b"\"")?;
        let body_root = find_field(input, b"\"body_root\":\"", b"\"")?;
        let aggregate_pub_key = find_field(input, b"\"aggregate_pubkey\":\"", b"\"")?;

        let beacon = Beacon {
            slot: std::str::from_utf8(&input[slot.0..slot.1])
                .ok()?
                .parse()
                .ok()?,
            proposer_index: std::str::from_utf8(&input[proposer_index.0..proposer_index.1])
                .ok()?
                .parse()
                .ok()?,
            parent_root: hex_to_b256(&input[parent_root.0..parent_root.1]),
            state_root: hex_to_b256(&input[state_root.0..state_root.1]),
            body_root: hex_to_b256(&input[body_root.0..body_root.1]),
        };

        let current_sync_committee = CurrentSyncCommittee {
            pub_keys: Self::parse_pub_keys_array(input)?
                .iter()
                .map(|&(start, end)| hex_to_b256(&input[start..end]))
                .collect(),
            aggregate_pubkey: hex_to_b256(&input[aggregate_pub_key.0..aggregate_pub_key.1]),
        };

        let current_sync_committee_branch: Vec<B256> = Self::parse_committee_branch_array(input)?
            .iter()
            .map(|&(start, end)| hex_to_b256(&input[start..end]))
            .collect();

        let header = Header { beacon };

        Some(LightClientBootstrap {
            version: std::str::from_utf8(&input[version.0..version.1]).ok()?,
            header,
            current_sync_committee,
            current_sync_committee_branch,
            code: None,
        })
    }

    pub fn parse_pub_keys_array(data: &[u8]) -> Option<Vec<(usize, usize)>> {
        let start = memchr::memmem::find(data, b"\"pubkeys\":[")?;
        let mut pos = start + b"\"pubkeys\":[".len();
        let mut result = Vec::new();

        while data[pos] != b']' {
            while data[pos] != b'"' && data[pos] != b']' {
                pos += 1;
            }
            if data[pos] == b']' {
                break;
            }
            pos += 1;
            let committee_start = pos;

            while data[pos] != b'"' {
                pos += 1;
            }
            result.push((committee_start, pos));
            pos += 1;
        }

        Some(result)
    }

    pub fn parse_committee_branch_array(data: &[u8]) -> Option<Vec<(usize, usize)>> {
        let start = memchr::memmem::find(data, b"\"current_sync_committee_branch\":[")?;
        let mut pos = start + b"\"current_sync_committee_branch\":[".len();
        let mut result = Vec::new();

        while data[pos] != b']' {
            while data[pos] != b'"' && data[pos] != b']' {
                pos += 1;
            }
            if data[pos] == b']' {
                break;
            }
            pos += 1;
            let committee_start = pos;

            while data[pos] != b'"' {
                pos += 1;
            }
            result.push((committee_start, pos));
            pos += 1;
        }

        Some(result)
    }
}
