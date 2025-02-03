use super::find_field;

#[derive(Debug)]
pub struct RawFee {
    pub oldest_block: (usize, usize),
    pub reward: Vec<(usize, usize)>,
    pub base_fee_per_gas: Vec<(usize, usize)>,
    pub gas_used_ratio: Vec<(usize, usize)>,
    pub base_fee_per_blobs_gas: Vec<(usize, usize)>,
}

/// single data reply from the rpc call
#[derive(Debug)]
pub struct Generic {
    pub result_start: (usize, usize),
}

#[derive(Debug)]
pub struct RawJsonResponse<'a> {
    pub data: &'a [u8],
    pub result_start: usize,
    pub result_end: usize,
}

impl RawFee {
    #[inline]
    pub fn parse(input: &[u8]) -> Option<Self> {
        Some(Self {
            oldest_block: find_field(input, b"\"oldestBlock\":\"", b"\"")?,
            reward: parse_array(input, b"\"reward\":")?,
            base_fee_per_gas: parse_array(input, b"\"baseFeePerGas\":")?,
            gas_used_ratio: parse_array(input, b"\"gasUsedRatio\":")?,
            base_fee_per_blobs_gas: parse_array(input, b"\"baseFeePerBlobGas\":")?,
        })
    }
}

#[inline]
fn parse_array(data: &[u8], field: &[u8]) -> Option<Vec<(usize, usize)>> {
    let start = memchr::memmem::find(data, field)? + field.len();
    let mut pos = start;
    let mut result = Vec::new();

    while data[pos] != b']' {
        while data[pos] != b'"' && data[pos] != b']' {
            pos += 1;
        }
        if data[pos] == b']' {
            break;
        }

        pos += 1;
        let value_start = pos;

        while data[pos] != b'"' {
            pos += 1;
        }
        result.push((value_start, pos));
        pos += 1;
    }

    Some(result)
}

impl Generic {
    #[inline]
    pub fn parse(input: &[u8]) -> Option<Self> {
        Some(Self {
            result_start: find_field(input, b"\"result\":\"", b"\"")?,
        })
    }
}
