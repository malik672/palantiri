use super::lib::{find_field, hex_to_b256, hex_to_u256, hex_to_u64, unsafe_hex_to_address, unsafe_hex_to_b256};
use super::types::Block;

// Field indices for fast lookup
const FIELD_COUNT: usize = 17;
const NUMBER: usize = 0;
const HASH: usize = 1;
const PARENT_HASH: usize = 2;
const UNCLES_HASH: usize = 3;
const AUTHOR: usize = 4;
const STATE_ROOT: usize = 5;
const TRANSACTIONS_ROOT: usize = 6;
const RECEIPTS_ROOT: usize = 7;
const LOGS_BLOOM: usize = 8;
const DIFFICULTY: usize = 9;
const GAS_LIMIT: usize = 10;
const GAS_USED: usize = 11;
const TIMESTAMP: usize = 12;
const EXTRA_DATA: usize = 13;
const MIX_HASH: usize = 14;
const NONCE: usize = 15;
const BASE_FEE_PER_GAS: usize = 16;

// Field patterns for search
static FIELD_PATTERNS: [(&[u8], &[u8]); FIELD_COUNT] = [
    (b"\"number\":\"", b"\""),
    (b"\"hash\":\"", b"\""),
    (b"\"parentHash\":\"", b"\""),
    (b"\"sha3Uncles\":\"", b"\""),
    (b"\"miner\":\"", b"\""),
    (b"\"stateRoot\":\"", b"\""),
    (b"\"transactionsRoot\":\"", b"\""),
    (b"\"receiptsRoot\":\"", b"\""),
    (b"\"logsBloom\":\"", b"\""),
    (b"\"difficulty\":\"", b"\""),
    (b"\"gasLimit\":\"", b"\""),
    (b"\"gasUsed\":\"", b"\""),
    (b"\"timestamp\":\"", b"\""),
    (b"\"extraData\":\"", b"\""),
    (b"\"mixHash\":\"", b"\""),
    (b"\"nonce\":\"", b"\""),
    (b"\"baseFeePerGas\":\"", b"\""),
];

#[derive(Debug)]
pub struct RawBlock<'a> {
    data: &'a [u8],
    fields: [(usize, usize); FIELD_COUNT],
    fields_present: u32,
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
    #[inline]
    pub fn parse(input: &'a [u8]) -> Option<Self> {
        // Use a bitfield to track which fields are present (faster than Option<>)
        let mut fields_present: u32 = 0;
        let mut fields = [(0, 0); FIELD_COUNT];
        
        // Parse all fields in one batch operation
        for (idx, &(prefix, suffix)) in FIELD_PATTERNS.iter().enumerate() {
            if let Some(range) = find_field(input, prefix, suffix) {
                fields[idx] = range;
                fields_present |= 1 << idx;
            }
        }
        
        /*This should be returned one day but for a perfect block we never hit this */
        // if (fields_present & ((1 << NUMBER) | (1 << HASH))) != ((1 << NUMBER) | (1 << HASH)) {
        //     println!("Missing required fields: number or hash");
        //     return None;
        // }

        
        // Estimate transaction count based on response size - newer blocks are much larger
        let tx_capacity = if input.len() > 500_000 {
            1500  // Large modern blocks can have 300+ transactions
        } else if input.len() > 200_000 {
            800   // Medium blocks
        } else {
            400   // Smaller blocks
        };
        
        let mut transactions = Vec::with_capacity(tx_capacity);
        let mut uncles = Vec::with_capacity(2); 
        
        // Parse transaction array - only if we need it
        Self::parse_transactions_array(input, &mut transactions);
        Self::parse_uncles_array(input, &mut uncles);
        
        Some(Self {
            data: input,
            fields,
            fields_present,
            transactions,
            uncles,
        })
    }

    #[inline]
    fn parse_transactions_array(data: &[u8], result: &mut Vec<(usize, usize)>) {
        // Use our existing find_field to locate the transactions array start
        if let Some(start) = memchr::memmem::find(data, b"\"transactions\":[") {
            let mut pos = start + b"\"transactions\":[".len();
            
            // Single-pass extraction of all transaction hashes
            let len = data.len();
            while pos < data.len() {
                // Skip whitespace and commas
                while pos < len && (data[pos] == b' ' || data[pos] == b',' || data[pos] == b'\n') {
                    pos += 1;
                }
                
                if pos >= len || data[pos] == b']' {
                    break;
                }
                
                // Only process string values (transaction hashes)
                if data[pos] == b'"' {
                    pos += 1; 
                    let tx_start = pos;
                    
                    // Find closing quote efficiently using memchr
                    if let Some(end_offset) = memchr::memchr(b'"', &data[pos..]) {
                        let tx_end = pos + end_offset;
                        result.push((tx_start, tx_end));
                        pos = tx_end + 1; 
                    } else {
                        break;
                    }
                } else {
                    // Not a string, might be an object or something else
                    // Skip until next comma or closing bracket
                    while pos < len && data[pos] != b',' && data[pos] != b']' {
                        pos += 1;
                    }
                }
            }
        }
    }

    #[inline]
    fn parse_uncles_array(data: &[u8], result: &mut Vec<(usize, usize)>) {
        // Similar approach as transactions array but for uncles
        if let Some(start) = memchr::memmem::find(data, b"\"uncles\":[") {
            let mut pos = start + b"\"uncles\":[".len();
            
            while pos < data.len() {
                // Skip whitespace and commas
                while pos < data.len() && (data[pos] == b' ' || data[pos] == b',' || data[pos] == b'\n') {
                    pos += 1;
                }
                
                if pos >= data.len() || data[pos] == b']' {
                    break;
                }
                
                if data[pos] == b'"' {
                    pos += 1; // Skip opening quote
                    let uncle_start = pos;
                    
                    if let Some(end_offset) = memchr::memchr(b'"', &data[pos..]) {
                        let uncle_end = pos + end_offset;
                        result.push((uncle_start, uncle_end));
                        pos = uncle_end + 1; // Move past closing quote
                    } else {
                        break;
                    }
                } else {
                    while pos < data.len() && data[pos] != b',' && data[pos] != b']' {
                        pos += 1;
                    }
                }
            }
        }
    }

    #[inline]
    pub fn to_block(&self) -> Block {
        // Helper to check field presence and get slice
        let get_field = |idx: usize| -> &[u8] {
            if (self.fields_present & (1 << idx)) != 0 {
                let (start, end) = self.fields[idx];
                &self.data[start..end]
            } else {
                b"0x0" 
            }
        };
        
        Block {
            number: hex_to_u64(get_field(NUMBER)),
            hash: Some(hex_to_b256(get_field(HASH))),
            parent_hash: unsafe_hex_to_b256(get_field(PARENT_HASH)),
            uncles_hash: unsafe_hex_to_b256(get_field(UNCLES_HASH)),
            author: unsafe_hex_to_address(get_field(AUTHOR)),
            state_root: unsafe_hex_to_b256(get_field(STATE_ROOT)),
            transactions_root: unsafe_hex_to_b256(get_field(TRANSACTIONS_ROOT)),
            receipts_root: unsafe_hex_to_b256(get_field(RECEIPTS_ROOT)),
            logs_bloom: String::from_utf8_lossy(get_field(LOGS_BLOOM)).into_owned(),
            difficulty: hex_to_u64(get_field(DIFFICULTY)),
            gas_limit: hex_to_u256(get_field(GAS_LIMIT)),
            gas_used: hex_to_u256(get_field(GAS_USED)),
            timestamp: hex_to_u64(get_field(TIMESTAMP)),
            extra_data: String::from_utf8_lossy(get_field(EXTRA_DATA)).into_owned(),
            mix_hash: unsafe_hex_to_b256(get_field(MIX_HASH)),
            nonce: hex_to_u64(get_field(NONCE)),
            base_fee_per_gas: Some(hex_to_u256(get_field(BASE_FEE_PER_GAS))),
            prev_randao: None,
            // Process transactions and uncles as needed
            transactions: self.transactions
                .iter()
                .map(|&(s, e)| hex_to_b256(&self.data[s..e]))
                .collect(),
            uncles: self.uncles
                .iter()
                .map(|&(s, e)| hex_to_b256(&self.data[s..e ]))
                .collect(),
        }
    }
}

impl<'a> RawJsonResponse<'a> {
    #[inline]
    pub fn parse_block(input: &'a [u8]) -> Option<Self> {
        // Fast check for null result
        if memchr::memmem::find(input, b"\"result\":null").is_some() {
            return None;
        }
        
        // Find result object
        let start = memchr::memmem::find(input, b"\"result\":{")?;
        let start = start + 9;

        // Fast bracket matching to find the end of the result object
        let mut pos = start;
        let mut depth = 1;
        
        // Single-pass bracket matching - most efficient way
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
            result_end: pos - 1, // exclude closing }
        })
    }

    #[inline]
    pub fn block(&self) -> Option<RawBlock<'a>> {
        // Extract only the relevant part of the JSON
        let block_data = &self.data[self.result_start..=self.result_end];
        RawBlock::parse(block_data)
    }
}

#[inline]
pub fn parse_block(input: &[u8]) -> Option<Block> {
    // Fast path - direct pipeline
    RawJsonResponse::parse_block(input)
        .and_then(|r| r.block())
        .map(|block| block.to_block())
}