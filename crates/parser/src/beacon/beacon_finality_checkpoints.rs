use alloy_primitives::{B256, U64};

use crate::{find_field, hex_to_b256, hex_to_u64};

#[derive(Debug, Default)]
pub struct FinalityCheckpoint {
    pub epoch: U64,
    pub root: B256,
}

#[derive(Debug)]
pub struct BeaconFinalityData {
    pub previous_justified: FinalityCheckpoint,
    pub current_justified: FinalityCheckpoint,
    pub finalized: FinalityCheckpoint,
    pub code: Option<u16>,
}

impl<'a> BeaconFinalityData {
    pub fn parse(input: &'a [u8]) -> Option<Self> {
        if memchr::memmem::find(input, b"\"data\":").is_some() {
            let previous = memchr::memmem::find(input, b"\"previous_justified\":")?;
            let current = memchr::memmem::find(input, b"\"current_justified\":")?;
            let finalized = memchr::memmem::find(input, b"\"finalized\":")?;

            Some(Self {
                previous_justified: FinalityCheckpoint::parse(&input[previous..current])?,
                current_justified: FinalityCheckpoint::parse(&input[current..finalized])?,
                finalized: FinalityCheckpoint::parse(&input[finalized..])?,
                code: None,
            })
        } else {
            let code = find_field(input, b"\"code\":", b",")?;
            let code_str = std::str::from_utf8(&input[code.0..code.1]).ok()?;
            Some(Self{
                previous_justified: FinalityCheckpoint::default(),
                current_justified: FinalityCheckpoint::default(),
                finalized: FinalityCheckpoint::default(),
                code: Some(code_str.parse().ok()?),
            })
        }
    }
}

impl FinalityCheckpoint {
    pub fn parse(input: &[u8]) -> Option<Self> {
        let epoch = find_field(input, b"\"epoch\":", b",")?;
        let root = find_field(input, b"\"root\":", b"}")?;

        Some(Self {
            epoch: hex_to_u64(&input[epoch.0..epoch.1]),
            root: hex_to_b256(&input[root.0..root.1]),
        })
    }
}
