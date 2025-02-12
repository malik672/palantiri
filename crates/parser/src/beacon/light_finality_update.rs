use crate::{find_field, hex_to_b256, types::Beacon};
use alloy_primitives::{B256, U64};

#[derive(Debug, Default, Clone)]
pub struct FinalityUpdate {
    pub version: String,
    pub attested_header: Beacon,
    pub finalized_header: Beacon,
    pub finality_branch: Vec<B256>,
    pub sync_aggregate: SyncAggregate,
    pub signature_slot: U64,
    pub code: Option<u16>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SyncAggregate {
    pub sync_committee_bits: B256,
    pub sync_committee_signature: B256,
}

impl FinalityUpdate {
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
        let signature_slot = find_field(input, b"\"signature_slot\":\"", b"\"")?;
        let sync_committee_bits = find_field(input, b"\"sync_committee_bits\":\"", b"\"")?;
        let sync_committee_signature =
            find_field(input, b"\"sync_committee_signature\":\"", b"\"")?;

        let finalized_header = Self::parse_header(input, b"\"finalized_header\":")?;
        let attested_header = Self::parse_header(input, b"\"attested_header\":")?;

        let finality_branch: Vec<B256> = Self::finality_branch(input)?
            .iter()
            .map(|&(start, end)| hex_to_b256(&input[start..end]))
            .collect();

        let beacon_f = Self::parse_beacon(&input[finalized_header.0..finalized_header.1])?;
        let beacon_a = Self::parse_beacon(&input[attested_header.0..attested_header.1])?;

        let sync_committee_bits = &input[sync_committee_bits.0..sync_committee_bits.1];
        let sync_aggregate = SyncAggregate {
            sync_committee_bits: hex_to_b256(sync_committee_bits),
            sync_committee_signature: hex_to_b256(
                &input[sync_committee_signature.0..sync_committee_signature.1],
            ),
        };

        Some(FinalityUpdate {
            version: std::str::from_utf8(&input[version.0..version.1])
                .ok()?
                .to_string(),
            attested_header: beacon_a,
            finalized_header: beacon_f,
            finality_branch,
            sync_aggregate,
            signature_slot: std::str::from_utf8(&input[signature_slot.0..signature_slot.1])
                .unwrap()
                .parse()
                .unwrap(),
            code: None,
        })
    }

    fn parse_header(input: &[u8], key: &[u8]) -> Option<(usize, usize)> {
        let start = memchr::memmem::find(input, key)? + key.len();
        let mut depth = 0;
        let mut end = start;

        for (i, &b) in input[start..].iter().enumerate() {
            match b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = start + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        Some((start, end))
    }

    fn parse_beacon(input: &[u8]) -> Option<Beacon> {
        let slot = find_field(input, b"\"slot\":\"", b"\"")?;
        let proposer_index = find_field(input, b"\"proposer_index\":\"", b"\"")?;
        let parent_root = find_field(input, b"\"parent_root\":\"", b"\"")?;
        let state_root = find_field(input, b"\"state_root\":\"", b"\"")?;
        let body_root = find_field(input, b"\"body_root\":\"", b"\"")?;

        let slot = &input[slot.0..slot.1];
        let proposer_index = &input[proposer_index.0..proposer_index.1];
        Some(Beacon {
            slot: std::str::from_utf8(slot).unwrap().parse().unwrap(),
            proposer_index: std::str::from_utf8(proposer_index)
                .unwrap()
                .parse()
                .unwrap(),
            parent_root: hex_to_b256(&input[parent_root.0..parent_root.1]),
            state_root: hex_to_b256(&input[state_root.0..state_root.1]),
            body_root: hex_to_b256(&input[body_root.0..body_root.1]),
        })
    }

    fn finality_branch(data: &[u8]) -> Option<Vec<(usize, usize)>> {
        let start = memchr::memmem::find(data, b"\"finality_branch\":[")?;
        let mut pos = start + b"\"finality_branch\":[".len();
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
