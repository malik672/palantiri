use super::lib::{find_field, hex_to_address, hex_to_b256, hex_to_u256, hex_to_u64};
use super::types::Block;

#[derive(Debug)]
pub struct RawBlock<'a> {
    data: &'a [u8],
    number: (usize, usize),
    hash: (usize, usize),
    parent_hash: (usize, usize),
    uncles_hash: (usize, usize),
    author: (usize, usize),
    state_root: (usize, usize),
    transactions_root: (usize, usize),
    receipts_root: (usize, usize),
    logs_bloom: (usize, usize),
    difficulty: (usize, usize),
    gas_limit: (usize, usize),
    gas_used: (usize, usize),
    timestamp: (usize, usize),
    extra_data: (usize, usize),
    mix_hash: (usize, usize),
    nonce: (usize, usize),
    base_fee_per_gas: Option<(usize, usize)>,
    transactions: Vec<(usize, usize)>,
    uncles: Vec<(usize, usize)>,
}

#[derive(Debug)]
pub struct RawJsonResponse<'a> {
    pub data: &'a [u8],
    pub result_start: usize,
    pub result_end: usize,
}

impl<'a> RawBlock<'a> {
    /// Parses raw JSON bytes representing an Ethereum block and extracts byte ranges for all required block fields.
    ///
    /// Returns `Some(RawBlock)` if all mandatory fields and arrays are found; returns `None` if any required field is missing or the input is malformed. The `base_fee_per_gas` field is parsed as optional.
    ///
    /// # Examples
    ///
    /// ```
    /// let json = br#"{
    ///     "number":"0x1",
    ///     "hash":"0xabc",
    ///     "parentHash":"0xdef",
    ///     "sha3Uncles":"0x123",
    ///     "miner":"0x456",
    ///     "stateRoot":"0x789",
    ///     "transactionsRoot":"0x101",
    ///     "receiptsRoot":"0x112",
    ///     "logsBloom":"0x131",
    ///     "difficulty":"0x415",
    ///     "gasLimit":"0x161",
    ///     "gasUsed":"0x718",
    ///     "timestamp":"0x192",
    ///     "extraData":"0xabc",
    ///     "mixHash":"0xdef",
    ///     "nonce":"0x123",
    ///     "transactions":[],
    ///     "uncles":[]
    /// }"#;
    /// let raw_block = RawBlock::parse(json);
    /// assert!(raw_block.is_some());
    /// ```
    pub fn parse(input: &'a [u8]) -> Option<Self> {
        Some(Self {
            data: input,
            number: find_field(input, b"\"number\":\"", b"\"")?,
            hash: find_field(input, b"\"hash\":\"", b"\"")?,
            parent_hash: find_field(input, b"\"parentHash\":\"", b"\"")?,
            uncles_hash: find_field(input, b"\"sha3Uncles\":\"", b"\"")?,
            author: find_field(input, b"\"miner\":\"", b"\"")?,
            state_root: find_field(input, b"\"stateRoot\":\"", b"\"")?,
            transactions_root: find_field(input, b"\"transactionsRoot\":\"", b"\"")?,
            receipts_root: find_field(input, b"\"receiptsRoot\":\"", b"\"")?,
            logs_bloom: find_field(input, b"\"logsBloom\":\"", b"\"")?,
            difficulty: find_field(input, b"\"difficulty\":\"", b"\"")?,
            gas_limit: find_field(input, b"\"gasLimit\":\"", b"\"")?,
            gas_used: find_field(input, b"\"gasUsed\":\"", b"\"")?,
            timestamp: find_field(input, b"\"timestamp\":\"", b"\"")?,
            extra_data: find_field(input, b"\"extraData\":\"", b"\"")?,
            mix_hash: find_field(input, b"\"mixHash\":\"", b"\"")?,
            nonce: find_field(input, b"\"nonce\":\"", b"\"")?,
            base_fee_per_gas: find_field(input, b"\"baseFeePerGas\":\"", b"\""),
            transactions: Self::parse_transactions_array(input)?,
            uncles: Self::parse_uncles_array(input)?,
        })
    }

    #[inline]
    /// Converts the raw block data into a fully typed `Block` struct.
    ///
    /// Extracts and decodes all block fields from the referenced byte ranges, including optional fields such as `base_fee_per_gas`. Returns a `Block` with all fields populated from the raw JSON data.
    ///
    /// # Examples
    ///
    /// ```
    /// let raw_block = RawBlock::parse(json_bytes).unwrap();
    /// let block = raw_block.to_block();
    /// assert_eq!(block.number, 1234567);
    /// ```
    pub fn to_block(&self) -> Block {
        Block {
            number: hex_to_u64(&self.data[self.number.0..self.number.1]),
            hash: Some(hex_to_b256(&self.data[self.hash.0..self.hash.1])),
            parent_hash: hex_to_b256(&self.data[self.parent_hash.0..self.parent_hash.1]),
            uncles_hash: hex_to_b256(&self.data[self.uncles_hash.0..self.uncles_hash.1]),
            author: hex_to_address(&self.data[self.author.0..self.author.1]),
            state_root: hex_to_b256(&self.data[self.state_root.0..self.state_root.1]),
            transactions_root: hex_to_b256(
                &self.data[self.transactions_root.0..self.transactions_root.1],
            ),
            receipts_root: hex_to_b256(&self.data[self.receipts_root.0..self.receipts_root.1]),
            logs_bloom: String::from_utf8_lossy(&self.data[self.logs_bloom.0..self.logs_bloom.1])
                .to_string(),
            difficulty: hex_to_u64(&self.data[self.difficulty.0..self.difficulty.1]),
            gas_limit: hex_to_u256(&self.data[self.gas_limit.0..self.gas_limit.1]),
            gas_used: hex_to_u256(&self.data[self.gas_used.0..self.gas_used.1]),
            timestamp: hex_to_u64(&self.data[self.timestamp.0..self.timestamp.1]),
            extra_data: String::from_utf8_lossy(&self.data[self.extra_data.0..self.extra_data.1])
                .to_string(),
            mix_hash: hex_to_b256(&self.data[self.mix_hash.0..self.mix_hash.1]),
            nonce: hex_to_u64(&self.data[self.nonce.0..self.nonce.1]),
            base_fee_per_gas: self.base_fee_per_gas.map(|(start, end)| 
                hex_to_u256(&self.data[start..end])
            ),
            prev_randao: None,
            transactions: self
                .transactions
                .iter()
                .map(|&(s, e)| hex_to_b256(&self.data[s..e]))
                .collect(),
            uncles: self
                .uncles
                .iter()
                .map(|&(s, e)| hex_to_b256(&self.data[s..e]))
                .collect(),
        }
    }

    pub fn parse_transactions_array(data: &[u8]) -> Option<Vec<(usize, usize)>> {
        let start = memchr::memmem::find(data, b"\"transactions\":[")?;
        let mut pos = start + b"\"transactions\":[".len();
        let mut result = Vec::new();

        while data[pos] != b']' {
            while data[pos] != b'"' && data[pos] != b']' {
                pos += 1;
            }
            if data[pos] == b']' {
                break;
            }
            pos += 1;
            let tx_start = pos;

            while data[pos] != b'"' {
                pos += 1;
            }
            result.push((tx_start, pos));
            pos += 1;
        }

        Some(result)
    }

    pub fn parse_uncles_array(data: &[u8]) -> Option<Vec<(usize, usize)>> {
        let start = memchr::memmem::find(data, b"\"uncles\":[")?;
        let mut pos = start + b"\"uncles\":[".len();
        let mut result = Vec::new();

        while data[pos] != b']' {
            while data[pos] != b'"' && data[pos] != b']' {
                pos += 1;
            }
            if data[pos] == b']' {
                break;
            }
            pos += 1;
            let tx_start = pos;

            while data[pos] != b'"' {
                pos += 1;
            }
            result.push((tx_start, pos));
            pos += 1;
        }

        Some(result)
    }
}

impl<'a> RawJsonResponse<'a> {
    #[inline]
    pub fn parse_block(input: &'a [u8]) -> Option<Self> {
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
    /// Parses the `"result"` JSON object into a `RawBlock`.
    ///
    /// Returns `Some(RawBlock)` if the block data is present and successfully parsed, or `None` if parsing fails or required fields are missing.
    ///
    /// # Examples
    ///
    /// ```
    /// let json = br#"{"jsonrpc":"2.0","result":{"number":"0x1","hash":"0xabc", ... }, "id":1}"#;
    /// let response = RawJsonResponse::parse_block(json).unwrap();
    /// let raw_block = response.block();
    /// assert!(raw_block.is_some());
    /// ```
    pub fn block(&self) -> Option<RawBlock<'a>> {
       RawBlock::parse(&self.data[self.result_start..=self.result_end])
    }
}

/// Parses raw JSON bytes representing an Ethereum block and returns a fully typed `Block`.
///
/// Returns `None` if the input does not contain a valid block or required fields are missing.
///
/// # Examples
///
/// ```
/// let json = br#"{"jsonrpc":"2.0","id":1,"result":{"number":"0x1","hash":"0xabc...","parentHash":"0xdef...","transactions":[],"uncles":[]}}"#;
/// let block = parse_block(json);
/// assert!(block.is_some());
/// ```
pub fn parse_block(input: &[u8]) -> Option<Block> {
 RawJsonResponse::parse_block(input)
        .and_then(|r| r.block(), )
        .map(|tx| tx.to_block())
}
