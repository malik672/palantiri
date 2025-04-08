use alloy::hex;
use alloy::primitives::{Address, B256, U256, U64};

use super::types::RawJsonResponse;
use super::types::TransactionTx;

use super::lib::{find_field, hex_to_address, hex_to_b256, hex_to_u256, hex_to_u64};

#[derive(Debug)]
pub struct RawTx<'a> {
    data: &'a [u8],
    // Field positions (start, end)
    block_hash: (usize, usize),
    block_number: (usize, usize),
    hash: (usize, usize),
    input: (usize, usize),
    r: (usize, usize),
    s: (usize, usize),
    v: (usize, usize),
    gas: (usize, usize),
    gas_price: (usize, usize),
    from: (usize, usize),
    tx_index: (usize, usize),
    to: (usize, usize),
    value: (usize, usize),
    nonce: (usize, usize),
}

impl<'a> RawTx<'a> {
    #[inline]
    fn parse(input: &'a [u8]) -> Option<Self> {
        // Find positions of all fields
        Some(Self {
            data: input,
            block_hash: find_field(input, b"\"blockHash\":\"", b"\"")?,
            block_number: find_field(input, b"\"blockNumber\":\"", b"\"")?,
            hash: find_field(input, b"\"hash\":\"", b"\"")?,
            input: find_field(input, b"\"input\":\"", b"\"")?,
            r: find_field(input, b"\"r\":\"", b"\"")?,
            s: find_field(input, b"\"s\":\"", b"\"")?,
            v: find_field(input, b"\"v\":\"", b"\"")?,
            gas: find_field(input, b"\"gas\":\"", b"\"")?,
            gas_price: find_field(input, b"\"gasPrice\":\"", b"\"")?,
            from: find_field(input, b"\"from\":\"", b"\"")?,
            tx_index: find_field(input, b"\"transactionIndex\":\"", b"\"")?,
            to: find_field(input, b"\"to\":\"", b"\"")?,
            value: find_field(input, b"\"value\":\"", b"\"")?,
            nonce: find_field(input, b"\"nonce\":\"", b"\"")?,
        })
    }

    #[inline]
    pub fn block_hash(&self) -> B256 {
        let bytes = &self.data[self.block_hash.0..self.block_hash.1];
        hex_to_b256(&bytes[2..])
    }

    #[inline]
    pub fn from(&self) -> Address {
        let bytes = &self.data[self.from.0..self.from.1];
        hex_to_address(&bytes[2..])
    }

    #[inline]
    pub fn to_transaction(&self) -> TransactionTx {
        TransactionTx {
            block_hash: Some(self.block_hash()),
            block_number: Some(self.block_number()),
            hash: self.hash(),
            input: hex::encode_prefixed(&self.data[self.input.0..self.input.1]),
            r: self.r(),
            s: self.s(),
            v: self.v(),
            gas: self.gas(),
            from: self.from(),
            transaction_index: Some(self.transaction_index()),
            to: Some(self.to()),
            value: self.value(),
            nonce: self.nonce(),
            gas_price: self.gas_price(),
        }
    }

    #[inline]
    fn block_number(&self) -> U64 {
        let bytes = &self.data[self.block_number.0..self.block_number.1];
        hex_to_u64(&bytes[2..])
    }

    #[inline]
    fn hash(&self) -> B256 {
        let bytes = &self.data[self.hash.0..self.hash.1];
        hex_to_b256(&bytes[2..])
    }

    #[inline]
    fn r(&self) -> B256 {
        let bytes = &self.data[self.r.0..self.r.1];
        hex_to_b256(&bytes[2..])
    }

    #[inline]
    fn s(&self) -> B256 {
        let bytes = &self.data[self.s.0..self.s.1];
        hex_to_b256(&bytes[2..])
    }

    #[inline]
    fn v(&self) -> U64 {
        let bytes = &self.data[self.v.0..self.v.1];
        hex_to_u64(&bytes[2..])
    }

    #[inline]
    fn gas(&self) -> U256 {
        let bytes = &self.data[self.gas.0..self.gas.1];
        hex_to_u256(&bytes[2..])
    }

    #[inline]
    fn to(&self) -> Address {
        let bytes = &self.data[self.to.0..self.to.1];
        hex_to_address(&bytes[2..])
    }

    #[inline]
    fn transaction_index(&self) -> U64 {
        let bytes = &self.data[self.tx_index.0..self.tx_index.1];
        hex_to_u64(&bytes[2..])
    }

    #[inline]
    fn value(&self) -> U256 {
        let bytes = &self.data[self.value.0..self.value.1];
        hex_to_u256(&bytes[2..])
    }

    #[inline]
    fn nonce(&self) -> U64 {
        let bytes = &self.data[self.nonce.0..self.nonce.1];
        hex_to_u64(&bytes[2..])
    }

    #[inline]
    fn gas_price(&self) -> U64 {
        let bytes = &self.data[self.gas_price.0..self.gas_price.1];
        hex_to_u64(&bytes[2..])
    }
}

impl<'a> RawJsonResponse<'a> {
    #[inline]
    pub fn parse_tx(input: &'a [u8]) -> Option<Self> {
        if memchr::memmem::find(input, b"\"result\":null").is_some() {
            return None;
        }
        let start = memchr::memmem::find(input, b"\"result\":{")?;
        let start = start + 9;

        // Find matching closing }
        let mut pos = start;
        let mut depth = 1;
        while depth > 0 && pos < input.len() {
            match input[pos] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            pos += 1;
        }

        Some(Self {
            data: input,
            result_start: start,
            // exclude closing }
            result_end: pos - 1,
        })
    }

    #[inline]
    pub fn transaction(&self) -> Option<RawTx<'a>> {
        RawTx::parse(&self.data[self.result_start..=self.result_end])
    }
}

pub fn parse_transaction(input: &[u8]) -> Option<TransactionTx> {
    RawJsonResponse::parse_tx(input)
        .and_then(|r| r.transaction())
        .map(|tx| tx.to_transaction())
}
