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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Block {
    pub number: U64,
    #[serde(rename = "parentHash")]
    pub parent_hash: B256,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    pub difficulty: U64,
    #[serde(rename = "prevRandao")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_randao: Option<B256>,
    #[serde(rename = "gasLimit")]
    pub gas_limit: U256,
    #[serde(rename = "gasUsed")]
    pub gas_used: U256,
    pub timestamp: U64,
    #[serde(rename = "extraData")]
    pub extra_data: String,
    #[serde(rename = "mixHash")]
    #[serde(deserialize_with = "deserialize_optional_hex")]
    pub mix_hash: B256,
    pub nonce: U64,
    #[serde(rename = "baseFeePerGas")]
    pub base_fee_per_gas: Option<U256>,
    pub transactions: Vec<B256>,
    pub uncles: Vec<B256>,
}



#[derive(Debug)]
pub struct RawJsonResponse<'a> {
    pub data: &'a [u8],
    pub result_start: usize,
    pub result_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: B256,
    pub nonce: U64,
    #[serde(rename = "type")]
    pub type_tx: Option<U64>,
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
    pub gas_price: Option<U64>,
    #[serde(rename = "maxFeePerGas")]
    pub max_fee_per_gas: Option<U64>,
    #[serde(rename = "maxPriorityFeePerGas")]
    pub max_priority_fee_per_gas: Option<U64>,
    pub gas: U256,
    pub input: String,
    pub v: U64,
    pub r: B256,
    pub s: B256,
    pub access_list: Option<Vec<(Address, Vec<B256>)>>,
    pub init: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionTx {
    pub hash: B256,
    pub nonce: U64,
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
    pub gas_price: U64,
    pub gas: U256,
    pub input: String,
    pub v: U64,
    pub r: B256,
    pub s: B256,
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

#[derive(Debug, Serialize)]
pub struct TransactionRequest {
    pub from: Option<Address>,
    pub to: Option<Address>,
    pub gas: Option<U256>,
    pub gas_price: Option<U256>,
    pub value: Option<U256>,
    pub data: Option<String>,
    pub nonce: Option<U256>,
}

#[derive(Debug, Serialize)]
pub struct FilterParams {
    pub from_block: Option<U64>,
    pub to_block: Option<U64>,
    pub address: Option<Address>,
    pub topics: Option<Vec<Option<B256>>>,
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

/// ********** BEACON OF GONDOR ********** ///

/// Isssues
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncAggregate {
    pub sync_committee_bits: Vec<u8>,
    pub sync_committee_signature: Vec<B256>,
}

#[derive(Debug, Default)]
pub struct LightClientBootstrap<'a> {
    pub version: &'a str,
    pub header: Header,
    pub current_sync_committee: SyncCommittee,
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
pub struct SyncCommittee {
    pub pub_keys: Vec<B256>,
    pub aggregate_pubkey: B256,
}

#[derive(Debug, Default)]

pub struct LightClientHeader {
    pub header: Header,
}

#[derive(Debug, Default)]

pub struct LightClientUpdate {
    pub attested_header: LightClientHeader,
    pub next_sync_committee: SyncCommittee,
    pub next_sync_committee_branch: Vec<B256>,
    pub finalized_header: LightClientHeader,
    pub finality_branch: Vec<B256>,
    pub sync_aggregate: SyncAggregate,
    pub signature_slot: U64,
}

#[derive(Debug, Default)]

pub struct LightClientFinalityUpdate {
    pub attested_header: LightClientHeader,
    pub finalized_header: LightClientHeader,
    pub finality_branch: Vec<B256>,
    pub sync_aggregate: SyncAggregate,
    pub signature_slot: U64,
}

#[derive(Debug, Default)]

pub struct LightClientOptimisticUpdate {
    pub attested_header: LightClientHeader,
    pub sync_aggregate: SyncAggregate,
    pub signature_slot: U64,
}

#[derive(Debug, Default)]
pub struct LightClientStore {
    pub finalized_header: LightClientHeader,
    pub current_sync_committee: SyncCommittee,
    pub next_sync_committee: SyncCommittee,
    pub best_valid_update: Option<LightClientUpdate>,
    pub optimistic_header: LightClientHeader,
    pub previous_max_active_participants: U64,
    pub current_max_active_participants: U64,
}


pub static MAINNET_BOOTNODES : [&str; 4] = [
    "enode://d860a01f9722d78051619d1e2351aba3f43f943f6f00718d1b9baa4101932a1f5011f16bb2b1bb35db20d6fe28fa0bf09636d26a87d31de9ec6203eeedb1f666@18.138.108.67:30303",   // bootnode-aws-ap-southeast-1-001
    "enode://22a8232c3abc76a16ae9d6c3b164f98775fe226f0917b0ca871128a74a8e9630b458460865bab457221f1d448dd9791d24c4e5d88786180ac185df813a68d4de@3.209.45.79:30303",     // bootnode-aws-us-east-1-001
    "enode://2b252ab6a1d0f971d9722cb839a42cb81db019ba44c08754628ab4a823487071b5695317c8ccd085219c3a03af063495b2f1da8d18218da2d6a82981b45e6ffc@65.108.70.101:30303",   // bootnode-hetzner-hel
    "enode://4aeb4ab6c14b23e2c4cfdce879c04b0748a20d8e9b59e25ded2a08143e265c6c25936e74cbc8e641e3312ca288673d91f2f93f8e277de3cfa444ecdaaf982052@157.90.35.166:30303",   // bootnode-hetzner-fsn
];
