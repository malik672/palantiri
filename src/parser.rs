use alloy::hex;
use alloy_primitives::{Address, B256, U64};


use crate::types::Log;

#[derive(Debug)]
pub struct RawJsonResponse<'a> {
    pub data: &'a [u8],
    pub result_start: usize,
    pub result_end: usize,
}

#[derive(Debug)]
pub struct RawLog<'a> {
    data: &'a [u8],
    // Field positions
    address: (usize, usize),
    topics: [(usize, usize); 4],
    data_field: (usize, usize),
    block_number: (usize, usize),
    block_hash: (usize, usize),
    tx_hash: (usize, usize),
    tx_index: (usize, usize),
    log_index: (usize, usize),
}

#[derive(Debug)]
pub struct LogIterator<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> RawJsonResponse<'a> {
    #[inline]
    pub fn parse(input: &'a [u8]) -> Option<Self> {
        // Skip "jsonrpc" and "id" fields to find "result":[
        let start = memchr::memmem::find(input, b"\"result\":[")?;
        let start = start + 10;

        // Find the matching closing ] for the results array
        let mut depth = 1;
        let mut end = start;
        while depth > 0 && end < input.len() {
            match input[end] {
                b'[' => depth += 1,
                b']' => depth -= 1,
                _ => {}
            }
            end += 1;
        }

        Some(Self {
            data: input,
            result_start: start,
            result_end: end - 1,
        })
    }

    #[inline]
    pub fn logs(&self) -> LogIterator<'a> {
        LogIterator {
            data: &self.data[self.result_start..self.result_end],
            pos: 0,
        }
    }
}

impl<'a> Iterator for LogIterator<'a> {
    type Item = RawLog<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let data = self.data;
        let len = data.len();
    
        // Find next log start
        if let Some(found) = memchr::memchr(b'{', &data[self.pos..]) {
            let start = self.pos + found;
    
            // skip opening {
            let mut pos = start + 1; 
    
            // We know the exact structure:
            // Find topics array
            if let Some(topics_pos) = memchr::memmem::find(&data[pos..], b"\"topics\":[") {
                pos += topics_pos + 10; 
    
                // Process topics until we hit closing bracket
                loop {
                    // Skip whitespace/commas until quote or ]
                    while data[pos] != b'"' && data[pos] != b']' {
                        pos += 1;
                    }
    
                    // If we hit closing bracket, move past it and break
                    if data[pos] == b']' {
                        pos += 1;
                        break;
                    }

                    // Otherwise it's a quote, skip the topic
                    pos += 67;  
                    // Skip comma or closing bracket
                    pos += 1;   
                }
            }
    
            // Find closing } of the main object
            while pos < len && data[pos] != b'}' {
                pos += 1;
            }
    
            self.pos = pos + 1;
            return RawLog::parse(&data[start..=pos]);
        }
    
        None
    }
}

impl<'a> RawLog<'a> {
    #[inline]
    fn parse(input: &'a [u8]) -> Option<Self> {
        let address = find_field(input, b"\"address\":\"", b"\"")?;
        let topics = parse_topics_array(input)?;
        let data = find_field(input, b"\"data\":\"", b"\"")?;
        let block_number = find_field(input, b"\"blockNumber\":\"", b"\"")?;
        let block_hash = find_field(input, b"\"blockHash\":\"", b"\"")?;
        let tx_hash = find_field(input, b"\"transactionHash\":\"", b"\"")?;
        let tx_index = find_field(input, b"\"transactionIndex\":\"", b"\"")?;
        let log_index = find_field(input, b"\"logIndex\":\"", b"\"")?;

        Some(Self {
            data: input,
            address,
            topics,
            data_field: data,
            block_number,
            block_hash,
            tx_hash,
            tx_index,
            log_index,
        })
    }

    // Accessors that convert to final types
    #[inline]
    pub fn address(&self) -> Address {
        let bytes = &self.data[self.address.0..self.address.1];
        hex_to_address(&bytes[2..])
    }

    #[inline]
    pub fn topics(&self) -> [B256; 4] {
        let mut result = [B256::default(); 4];
        for (i, &(start, end)) in self.topics.iter().enumerate() {
            let bytes = &self.data[start..end];
            result[i] = hex_to_b256(&bytes[2..]);
        }
        result
    }

    #[inline]
    pub fn data(&self) -> &'a [u8] {
        &self.data[self.data_field.0..self.data_field.1]
    }

    #[inline]
    pub fn block_number(&self) -> U64 {
        let bytes = &self.data[self.block_number.0..self.block_number.1];
        hex_to_u64(&bytes[2..])
    }

    #[inline]
    pub fn block_hash(&self) -> B256 {
        let bytes = &self.data[self.block_hash.0..self.block_hash.1];
        hex_to_b256(&bytes[2..])
    }

    #[inline]
    pub fn transaction_hash(&self) -> B256 {
        let bytes = &self.data[self.tx_hash.0..self.tx_hash.1];
        hex_to_b256(&bytes[2..])
    }

    #[inline]
    pub fn transaction_index(&self) -> U64 {
        let bytes = &self.data[self.tx_index.0..self.tx_index.1];
        hex_to_u64(&bytes[2..])
    }

    #[inline]
    pub fn log_index(&self) -> U64 {
        let bytes = &self.data[self.log_index.0..self.log_index.1];
        hex_to_u64(&bytes[2..])
    }

    // Convert to standard Log struct if needed
    #[inline]
    pub fn to_log(&self) -> Log {
        Log {
            address: self.address(),
            topics: self.topics().to_vec(),
            data: hex::encode_prefixed(self.data()),
            block_number: Some(self.block_number()),
            block_hash: Some(self.block_hash()),
            transaction_hash: Some(self.transaction_hash()),
            transaction_index: Some(self.transaction_index()),
            log_index: Some(self.log_index()),
            //ISSSUE: This is not correct
            removed: Some(false),
        }
    }
}

#[inline]
fn find_field(data: &[u8], prefix: &[u8], suffix: &[u8]) -> Option<(usize, usize)> {
    let start = memchr::memmem::find(data, prefix)?;
    let start = start + prefix.len();
    let end = start + memchr::memmem::find(&data[start..], suffix)?;
    Some((start, end))
}

#[inline]
fn parse_topics_array(data: &[u8]) -> Option<[(usize, usize); 4]> {
    let pos = memchr::memmem::find(data, b"\"topics\":[")? + 10; // Hard-coded length
    let mut result = [(0, 0); 4];

    unsafe {
        let mut current = pos;
        for i in 0..4 {
            while data.get_unchecked(current) != &b'"' {
                current += 1;
            }
            // skip opening quote
            current += 1;

            // Each topic is exactly 66 bytes (including 0x)
            result[i] = (current, current + 66);
            // skip topic and closing quote
            current += 67;
        }
    }

    Some(result)
}

#[inline]
pub fn hex_to_address(hex: &[u8]) -> Address {
    let mut bytes = [0u8; 20];
    hex_to_bytes(hex, &mut bytes);
    Address::from_slice(&bytes)
}

#[inline]
pub fn hex_to_b256(hex: &[u8]) -> B256 {
    let mut bytes = [0u8; 32];
    hex_to_bytes(hex, &mut bytes);
    B256::from_slice(&bytes)
}

#[inline]
pub fn hex_to_u64(hex: &[u8]) -> U64 {
    let mut val = 0u64;
    for &b in hex {
        val = val * 16 + (b as char).to_digit(16).unwrap() as u64;
    }
    U64::from(val)
}

/// How do i guarante safety here: simply i don't, just belief
#[inline]
pub fn hex_to_bytes(hex: &[u8], out: &mut [u8]) {
    let len = out.len();
    let hex_ptr = hex.as_ptr();
    let out_ptr = out.as_mut_ptr();

    unsafe {
        for i in 0..len {
            let high = (*hex_ptr.add(i * 2) as char).to_digit(16).unwrap() as u8;
            let low = (*hex_ptr.add(i * 2 + 1) as char).to_digit(16).unwrap() as u8;
            *out_ptr.add(i) = (high << 4) | low;
        }
    }
}

pub fn parse_logs(input: &[u8]) -> Vec<Log> {
    let response = match RawJsonResponse::parse(input) {
        Some(r) => r,
        None => return Vec::new(),
    };

    response.logs().map(|l| l.to_log()).collect()
}
