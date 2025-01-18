//source: from the yellow paper

use std::str::FromStr;

use alloy::primitives::{Address, B256, U256, U64};
use serde::{Deserialize, Serialize};

//THIS IS A SCOPE TO TRACK THE HASH OF A BLOCK USING THE BLOCK NUMBER
//SINCE THE BLOCK HEADER DOES NOT CONTAIN THE HASH OF THE BLOCK, AND MAJORLY WE ARE USING THE BLOCK HEADER
///THIS IS A FORM OF BUFFER THAT STORES THE HASH  USING THE THE LAST DIGIT OF THE BLOCK NUMBER AS INDEX SO POSSIBLY IT CAN ONLY STORE 1-9 BLOCKS
pub static mut NUM_HASH_DATA: [B256; 10] = [B256::ZERO; 10];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlockHeader {
    #[serde(rename = "parentHash")]
    pub parent_hash: B256,
    #[serde(skip_serializing_if = "Option::is_none")]

    //Pun right
    #[serde(rename = "hash")]
    pub hash: Option<B256>,
    #[serde(rename = "sha3Uncles")]
    pub uncles_hash: B256,
    #[serde(rename = "miner")]
    pub author: Address,
    #[serde(rename = "stateRoot")]
    pub state_root: B256,
    #[serde(rename = "transactionsRoot")]
    pub transactions_root: B256,
    #[serde(rename = "receiptsRoot")]
    pub receipts_root: B256,
    #[serde(rename = "logsBloom")]
    pub logs_bloom: String,
    #[serde(deserialize_with = "deserialize_hex_number")]
    pub difficulty: u64,
    // Bellatrix additions
    #[serde(rename = "prevRandao")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_randao: Option<B256>,
    #[serde(deserialize_with = "deserialize_hex_number")]
    pub number: u64,
    #[serde(rename = "gasLimit")]
    pub gas_limit: U256,
    #[serde(rename = "gasUsed")]
    pub gas_used: U256,
    #[serde(deserialize_with = "deserialize_hex_number")]
    pub timestamp: u64,
    #[serde(rename = "extraData")]
    pub extra_data: String,
    #[serde(rename = "mixHash")]
    #[serde(deserialize_with = "deserialize_optional_hex")]
    pub mix_hash: B256,
    #[serde(deserialize_with = "deserialize_hex_number")]
    pub nonce: u64,
    #[serde(rename = "baseFeePerGas")]
    pub base_fee_per_gas: Option<U256>,

    // Capella additions
    #[serde(rename = "withdrawalsRoot")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withdrawls_root: Option<B256>,

    // Deneb additions
    #[serde(rename = "blobGasUsed")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_gas_used: Option<U64>,
    #[serde(rename = "excessBlobGas")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excess_blob_gas: Option<U64>,
    #[serde(rename = "parentBeaconBlockRoot")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_beacon_block_root: Option<B256>,
    #[serde(rename = "blobsHash")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blobs_hash: Option<B256>,

    // Altair additions(Altair is a fork of the Ethereum 2.0 beacon chai: this is funny ngl(personal reason))
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_aggregate: Option<SyncAggregate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncAggregate {
    pub sync_committee_bits: Vec<u8>,
    pub sync_committee_signature: Vec<u8>,
}


#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Block {
    #[serde(rename = "number")]
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub uncles: Vec<BlockHeader>,
    #[serde(rename = "withdrawals")]
    pub withdrawals: Vec<Withdrawal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: B256,
    pub nonce: U256,
    #[serde(rename = "blockHash")]
    pub block_hash: Option<B256>,
    #[serde(rename = "number")]
    pub block_number: Option<U64>,
    #[serde(rename = "transactionIndex")]
    pub transaction_index: Option<U64>,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    #[serde(rename = "gasPrice")]
    pub gas_price: Option<U256>,
    #[serde(rename = "maxFeePerGas")]
    pub max_fee_per_gas: Option<U256>,
    #[serde(rename = "maxPriorityFeePerGas")]
    pub max_priority_fee_per_gas: Option<U256>,
    pub gas: U256,
    pub input: Vec<u8>,
    pub v: u64,
    pub r: U256,
    pub s: U256,
    pub access_list: Option<Vec<(Address, Vec<B256>)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub address: Address,
    pub topics: Vec<B256>,
    pub data: String,
    #[serde(rename = "blockNumber")]
    pub block_number: Option<U64>,
    #[serde(rename = "blockHash")]
    pub block_hash: Option<B256>,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: Option<B256>,
    #[serde(rename = "transactionIndex")]
    pub transaction_index: Option<U64>,
    #[serde(rename = "logIndex")]
    pub log_index: Option<U64>,
    pub removed: Option<bool>,
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Withdrawal {
    pub address: Address,
    pub amount: U256,
    pub index: U64,
    #[serde(rename = "validatorIndex")]
    pub validator_index: U64,
}


fn deserialize_hex_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    u64::from_str_radix(s.trim_start_matches("0x"), 16).map_err(serde::de::Error::custom)
}

fn deserialize_optional_hex<'de, D>(deserializer: D) -> Result<B256, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    if s == "0x" {
        return Ok(B256::ZERO);
    }

    B256::from_str(s.trim_start_matches("0x")).map_err(serde::de::Error::custom)
}
