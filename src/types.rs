//source: from the yellow paper


use alloy::primitives::{Address, B256, U256, U64};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub parent_hash: B256,
    pub uncles_hash: B256,
    pub author: Address,
    pub state_root: B256,
    pub transactions_root: B256,
    pub receipts_root: B256,
    ///ISSUE
    pub logs_bloom: B256,
    pub difficulty: U256,
    pub number: u64,
    pub gas_limit: U256,
    pub gas_used: U256,
    pub timestamp: u64,
    pub extra_data: Vec<u8>,
    pub mix_hash: B256,
    pub nonce: u64,
    pub base_fee_per_gas: Option<U256>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub uncles: Vec<BlockHeader>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: B256,
    pub nonce: U256,
    pub block_hash: Option<B256>,
    pub block_number: Option<U64>,
    pub transaction_index: Option<U64>,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub gas_price: Option<U256>,
    pub max_fee_per_gas: Option<U256>,
    pub max_priority_fee_per_gas: Option<U256>,
    pub gas: U256,
    pub input: Vec<u8>,
    pub v: u64,
    pub r: U256,
    pub s: U256,
}

impl Block {
    pub fn hash(&self) -> B256 {
        // TODO: Implement block hash calculation
        B256::ZERO
    }
}