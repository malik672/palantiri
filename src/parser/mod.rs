use alloy_primitives::{Address, B256, U256, U64};

pub mod block_parser;
pub mod log_parser;
pub mod tx_parser;

#[inline]
pub fn hex_to_address(hex: &[u8]) -> Address {
    let mut bytes = [0u8; 20];
    // Skip 0x if present
    let hex = if hex.len() >= 2 && hex[0] == b'0' && (hex[1] == b'x' || hex[1] == b'X') {
        &hex[2..]
    } else {
        hex
    };

    let hex_ptr = hex.as_ptr();
    let out_ptr = bytes.as_mut_ptr();

    unsafe {
        for i in 0..20 {
            let high = (*hex_ptr.add(i * 2) as char).to_digit(16).unwrap_or(0) as u8;
            let low = (*hex_ptr.add(i * 2 + 1) as char).to_digit(16).unwrap_or(0) as u8;
            *out_ptr.add(i) = (high << 4) | low;
        }
    }

    Address::from_slice(&bytes)
}

#[inline]
pub fn hex_to_b256(hex: &[u8]) -> B256 {
    let mut bytes = [0u8; 32];
    // Skip 0x if present
    let hex = if hex.len() >= 2 && hex[0] == b'0' && (hex[1] == b'x' || hex[1] == b'X') {
        &hex[2..]
    } else {
        hex
    };

    let hex_ptr = hex.as_ptr();
    let out_ptr = bytes.as_mut_ptr();

    unsafe {
        for i in 0..32 {
            let high = (*hex_ptr.add(i * 2) as char).to_digit(16).unwrap_or(0) as u8;
            let low = (*hex_ptr.add(i * 2 + 1) as char).to_digit(16).unwrap_or(0) as u8;
            *out_ptr.add(i) = (high << 4) | low;
        }
    }

    B256::from_slice(&bytes)
}

#[inline]
pub fn hex_to_u64(hex: &[u8]) -> U64 {
    let mut bytes = [0u8; 8];

    // Calculate actual number of bytes from hex length
    let hex_len = hex.len();
    let byte_len = hex_len / 2;

    let start_idx = if byte_len > 8 { 0 } else { 8 - byte_len };

    unsafe {
        let hex_ptr = hex.as_ptr();
        let out_ptr = bytes.as_mut_ptr();

        for i in 0..byte_len {
            let high = (*hex_ptr.add(i * 2) as char).to_digit(16).unwrap_or(0) as u8;
            let low = (*hex_ptr.add(i * 2 + 1) as char).to_digit(16).unwrap_or(0) as u8;
            *out_ptr.add(start_idx + i) = (high << 4) | low;
        }
    }

    U64::from_be_bytes(bytes)
}

pub fn hex_to_bytes(hex: &[u8], out: &mut [u8]) -> Result<(), &'static str> {
    let out_len = out.len();
    if hex.len() < out_len * 2 {
        return Err("hex string too short");
    }

    let hex_ptr = hex.as_ptr();
    let out_ptr = out.as_mut_ptr();

    unsafe {
        for i in 0..out_len {
            let high = match (*hex_ptr.add(i * 2) as char).to_digit(16) {
                Some(h) => h as u8,
                None => return Err("invalid hex character"),
            };

            let low = match (*hex_ptr.add(i * 2 + 1) as char).to_digit(16) {
                Some(l) => l as u8,
                None => return Err("invalid hex character"),
            };

            *out_ptr.add(i) = (high << 4) | low;
        }
    }

    Ok(())
}

#[inline]
pub fn hex_to_u256(hex: &[u8]) -> U256 {
    let mut bytes = [0u8; 32];

    // Skip 0x if present
    let hex = if hex.len() >= 2 && hex[0] == b'0' && (hex[1] == b'x' || hex[1] == b'X') {
        &hex[2..]
    } else {
        hex
    };

    // Calculate actual number of bytes from hex length
    let hex_len = hex.len();
    let byte_len = hex_len / 2;

    let start_idx = if byte_len > 32 { 0 } else { 32 - byte_len };

    unsafe {
        let hex_ptr = hex.as_ptr();
        let out_ptr = bytes.as_mut_ptr();

        for i in 0..byte_len {
            let high = (*hex_ptr.add(i * 2) as char).to_digit(16).unwrap_or(0) as u8;
            let low = (*hex_ptr.add(i * 2 + 1) as char).to_digit(16).unwrap_or(0) as u8;
            *out_ptr.add(start_idx + i) = (high << 4) | low;
        }
    }

    U256::from_be_bytes(bytes)
}

#[inline]
pub fn find_field(data: &[u8], prefix: &[u8], suffix: &[u8]) -> Option<(usize, usize)> {
    let start = memchr::memmem::find(data, prefix)?;
    let start = start + prefix.len();
    let end = start + memchr::memmem::find(&data[start..], suffix)?;
    Some((start, end))
}
