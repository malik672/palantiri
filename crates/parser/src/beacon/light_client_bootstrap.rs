use alloy_primitives::{B256, U64};

#[derive(Debug, Default)]
pub struct LightClientBootstrap<'a> {
    pub version: &'a str,
    pub header: Header,
    pub current_sync_committee: CurrentSyncCommittee,
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
    
}

#[derive(Debug, Default)]
pub struct CurrentSyncCommittee {
    pub pub_keys: Vec<B256>,
    pub aggregate_pubkey: B256,
}
