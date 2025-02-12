use alloy_primitives::{B256, U64};

use crate::{find_field, hex_to_b256,  types::Beacon};

#[derive(Debug, Default, Clone)]
pub struct LightOptimisticUpdate {
    pub version: String,
    pub attested_header: Beacon,
    pub sync_aggregate: SyncAggregate,
    pub signature_slot: U64,
    pub code: Option<u16>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SyncAggregate {
    pub sync_committee_bits: B256,
    pub sync_committee_signature: B256,
}

impl LightOptimisticUpdate {
    pub fn parse(input: &[u8]) -> Option<Self> {
        if let Some(_pos) = memchr::memmem::find(input, b"\"code\":") {
            let code = find_field(input, b"\"code\":", b"}")?;
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
        let sync_committee_bits = find_field(input, b"\"sync_committee_bits\":\"", b"\"")?;
        let sync_committee_signatures =
            find_field(input, b"\"sync_committee_signature\":\"", b"\"")?;

        let sync_aggregate = SyncAggregate {
            sync_committee_bits: hex_to_b256(&input[sync_committee_bits.0..sync_committee_bits.1]),
            sync_committee_signature: hex_to_b256(
                &input[sync_committee_signatures.0..sync_committee_signatures.1],
            ),
        };

        let signature_slot = find_field(&input, b"\"signature_slot\":\"", b"\"")?;

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

        Some(LightOptimisticUpdate {
            version: std::str::from_utf8(&input[version.0..version.1])
                .ok()?
                .to_string(),
            attested_header: beacon,
            sync_aggregate,
            signature_slot: std::str::from_utf8(&input[signature_slot.0..signature_slot.1]).unwrap().parse().unwrap(),
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
