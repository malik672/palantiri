use alloy_primitives::B256;

use crate::{find_field, hex_to_address, hex_to_b256, hex_to_u256, types::{Beacon, Execution, LightOptimisticUpdate, SyncAggregate}};


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

        let execution = Self::parse_header(input, b"\"execution\":")?;
        let execution = Self::parse_execution(&input[execution.0..execution.1])?;

        
        let execution_branch: Vec<B256> = Self::execution_branch(input)?
            .iter()
            .map(|&(start, end)| hex_to_b256(&input[start..end]))
            .collect();


        Some(LightOptimisticUpdate {
            version: std::str::from_utf8(&input[version.0..version.1])
                .ok()?
                .to_string(),
            attested_header: beacon,
            sync_aggregate,
            signature_slot: std::str::from_utf8(&input[signature_slot.0..signature_slot.1]).unwrap().parse().unwrap(),
            code: None,
            execution_branch,
            execution,
        })
    }

    fn parse_execution(input: &[u8]) -> Option<Execution> {
        let parent_hash = find_field(input, b"\"parent_hash\":\"", b"\"")?;
        let fee_recipient = find_field(input, b"\"fee_recipient\":\"", b"\"")?;
        let state_root = find_field(input, b"\"state_root\":\"", b"\"")?;
        let receipt_root = find_field(input, b"\"receipt_root\":\"", b"\"")?;
        let logs_bloom = find_field(input, b"\"logs_bloom\":\"", b"\"")?;
        let prev_randao = find_field(input, b"\"prev_randao\":\"", b"\"")?;
        let block_number = find_field(input, b"\"block_number\":\"", b"\"")?;
        let gas_limit = find_field(input, b"\"gas_limit\":\"", b"\"")?;
        let gas_used = find_field(input, b"\"gas_used\":\"", b"\"")?;
        let timestamp = find_field(input, b"\"timestamp\":\"", b"\"")?;
        let extra_data = find_field(input, b"\"extra_data\":\"", b"\"")?;
        let base_fee_per_gas = find_field(input, b"\"base_fee_per_gas\":\"", b"\"")?;
        let excess_blob_gas = find_field(input, b"\"excess_blob_gas\":\"", b"\"")?;
        let block_hash = find_field(input, b"\"block_hash\":\"", b"\"")?;
        let transactions_root = find_field(input, b"\"transactions_root\":\"", b"\"")?;
        let withdrawals_root = find_field(input, b"\"withdrawals_root\":\"", b"\"")?;

        let block_number = &input[block_number.0..block_number.1];
        let gas_used = &input[gas_used.0..gas_used.1];
        let timestamp = &input[timestamp.0..timestamp.1];
        let excess_blob_gas = &input[excess_blob_gas.0..excess_blob_gas.1];
        
        Some(Execution{
            parent_hash: hex_to_b256(&input[parent_hash.0..parent_hash.1]),
            fee_recipient: hex_to_address(&input[fee_recipient.0..fee_recipient.1]),
            state_root: hex_to_b256(&input[state_root.0..state_root.1]),
            receipts_root: hex_to_b256(&input[receipt_root.0..receipt_root.1]),
            logs_bloom: std::str::from_utf8(&input[logs_bloom.0..logs_bloom.1]).unwrap().to_string(),
            prev_randao: hex_to_b256(&input[prev_randao.0..prev_randao.1]),
            block_number: std::str::from_utf8(block_number).unwrap().parse().unwrap(),
            gas_limit: hex_to_u256(&input[gas_limit.0..gas_limit.1]),
            gas_used: std::str::from_utf8(gas_used).unwrap().parse().unwrap(),
            timestamp: std::str::from_utf8(timestamp).unwrap().parse().unwrap(),
            extra_data: std::str::from_utf8(&input[extra_data.0..extra_data.1]).unwrap().to_string(),
            base_fee_per_gas: hex_to_u256(&input[base_fee_per_gas.0..base_fee_per_gas.1]),
            excess_blob_gas:  std::str::from_utf8(excess_blob_gas).unwrap().parse().unwrap(),
            block_hash: hex_to_b256(&input[block_hash.0..block_hash.1]),
            transactions_root: hex_to_b256(&input[transactions_root.0..transactions_root.1]),
            withdrawals_root: hex_to_b256(&input[withdrawals_root.0..withdrawals_root.1]),
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


    fn execution_branch(data: &[u8]) -> Option<Vec<(usize, usize)>> {
        let start = memchr::memmem::find(data, b"\"execution_branch\":[")?;
        let mut pos = start + b"\"execution_branch\":[".len();
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
