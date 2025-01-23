use super::{find_field, hex_to_address, hex_to_b256, hex_to_u256, hex_to_u64};

#[derive(Debug)]
pub struct RawFee {
    oldest_block: (usize, usize),
    reward: Vec<(usize, usize)>,
    bae_fee_per_gas: Vec<(usize, usize)>,
    gas_used_ratio: Vec<(usize, usize)>,
    base_fee_per_blobs_gas: Vec<(usize, usize)>,
}