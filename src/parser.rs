use alloy::hex;
use alloy_primitives::{Address, B256, U64};
use core::{marker::PhantomData, mem};
use memchr::memmem;

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
    // #[cfg(target_arch = "aarch64")]
    // #[target_feature(enable = "neon")]
    // pub unsafe fn parse(input: &'a [u8]) -> Option<Self> {
    //     use std::arch::aarch64::{
    //         vceqq_u8, vdupq_n_u8, vget_lane_u64, vget_low_u8, vgetq_lane_u64, vld1q_u8,
    //         vreinterpret_u64_u8, vreinterpretq_u64_u8,
    //     };

    //     unsafe {
    //         // First find "result":[ using SIMD
    //         let pattern = b"\"result\":[";
    //         let pattern_vec = vld1q_u8(pattern.as_ptr());
    //         let mut pos = 0;

    //         // Process 16 bytes at a time
    //         while pos + 16 <= input.len() {
    //             let chunk = vld1q_u8(input[pos..].as_ptr());

    //             // Compare current chunk with pattern
    //             let eq_mask = vceqq_u8(chunk, pattern_vec);
    //             let mask = vget_lane_u64(vreinterpret_u64_u8(vget_low_u8(eq_mask)), 0);

    //             if mask != 0 {
    //                 let start = pos + (mask.trailing_zeros() as usize);
    //                 if start + 10 > input.len() {
    //                     return None;
    //                 }

    //                 // Now find matching brackets using SIMD
    //                 let mut cursor = start + 10; // Skip "result":[
    //                 let mut depth = 1;

    //                 // Process brackets 16 bytes at a time
    //                 let open_bracket = vdupq_n_u8(b'[');
    //                 let close_bracket = vdupq_n_u8(b']');

    //                 while cursor + 16 <= input.len() && depth > 0 {
    //                     let data = vld1q_u8(input[cursor..].as_ptr());

    //                     // Find all brackets in current chunk
    //                     let opens = vceqq_u8(data, open_bracket);
    //                     let closes = vceqq_u8(data, close_bracket);

    //                     // Convert to bitmasks
    //                     let open_mask = vgetq_lane_u64(vreinterpretq_u64_u8(opens), 0) as u16;
    //                     let close_mask = vgetq_lane_u64(vreinterpretq_u64_u8(closes), 0) as u16;

    //                     // Count brackets
    //                     depth += open_mask.count_ones() as i32;
    //                     depth -= close_mask.count_ones() as i32;

    //                     if depth == 0 {
    //                         let close_pos = cursor + close_mask.trailing_zeros() as usize;
    //                         return Some(Self {
    //                             data: input,
    //                             result_start: start + 10,
    //                             result_end: close_pos,
    //                         });
    //                     }

    //                     cursor += 16;
    //                 }

    //                 // Handle remaining bytes sequentially
    //                 while depth > 0 && cursor < input.len() {
    //                     match input[cursor] {
    //                         b'[' => depth += 1,
    //                         b']' => depth -= 1,
    //                         _ => {}
    //                     }
    //                     cursor += 1;

    //                     if depth == 0 {
    //                         return Some(Self {
    //                             data: input,
    //                             result_start: start + 10,
    //                             result_end: cursor - 1,
    //                         });
    //                     }
    //                 }
    //             }
    //             pos += 1;
    //         }
    //         None
    //     }
    // }

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
        // Skip whitespace, commas until next {
        while self.pos < self.data.len() && self.data[self.pos] != b'{' {
            self.pos += 1;
        }
        if self.pos >= self.data.len() {
            return None;
        }

        let start = self.pos;
        let mut depth = 0;
        while self.pos < self.data.len() {
            match self.data[self.pos] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            self.pos += 1;
        }

        let log_slice = &self.data[start..=self.pos];
        self.pos += 1;

        RawLog::parse(log_slice)
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
    let topics_start = memchr::memmem::find(data, b"\"topics\":[")? + b"\"topics\":[".len();
    let mut result = [(0, 0); 4];
    let mut pos = topics_start;
    let mut idx = 0;

    while idx < 4 {
        // Skip until opening quote
        while data[pos] != b'"' {
            pos += 1;
        }
        pos += 1;
        let start = pos;

        // Find closing quote
        while data[pos] != b'"' {
            pos += 1;
        }
        result[idx] = (start, pos);
        idx += 1;
        pos += 1;
    }

    Some(result)
}

// Fast hex parsing functions
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

#[inline]
// pub fn hex_to_bytes(hex: &[u8], out: &mut [u8]) {
//     for i in 0..out.len() {
//         let high = (hex[i * 2] as char).to_digit(16).unwrap() as u8;
//         let low = (hex[i * 2 + 1] as char).to_digit(16).unwrap() as u8;
//         out[i] = (high << 4) | low;
//     }
// }
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
    let response = match unsafe { RawJsonResponse::parse(input) } {
        Some(r) => r,
        None => return Vec::new(),
    };

    response.logs().map(|l| l.to_log()).collect()
}
