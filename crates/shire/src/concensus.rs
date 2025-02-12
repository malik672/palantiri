use std::sync::{Arc, RwLock};

use alloy::primitives::{Address, BlockHash, B256};
use chrono::{TimeZone, Utc};
use mordor::SlotSynchronizer;

use palantiri::rpc::RpcClient;

use parser::types::BlockHeader;

#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    pub chain_id: u64,
    pub finalized_block_number: u64,
    pub genesis_hash: B256,
    pub finalized_block_hash: B256,
    pub sync_period: u64,
    pub min_sync_comitee: u64,
}

#[derive(Debug)]
pub struct ConsensusState {
    pub current_block: u64,
    pub finalized_block: BlockHash,
    pub finalized_block_number: u64,
    pub sync_status: SyncStatus,
    pub min_sync_committee_participants: u64,
}
#[derive(Debug)]
pub struct ConsensusImpl {
    pub config: ConsensusConfig,
    state: RwLock<ConsensusState>,
    rpc: Arc<RpcClient>,
    slot_sync: SlotSynchronizer,
}

#[derive(Debug)]
pub struct SyncAggregate {
    pub sync_committee_bits: u64,
    pub sync_committee_signature: Vec<u8>,
}

#[derive(Debug)]
pub struct SyncCommittee {
    pub period: u64,
    // pub pubkeys: Vec<PublicKey>,
    // pub aggregate_pubkey: PublicKey,
}

#[derive(Debug)]
pub struct FinalityUpdate {
    pub attested_header: BlockHeader,
    pub finalized_header: BlockHeader,
    pub finality_branch: Vec<B256>,
    pub sync_aggregate: SyncAggregate,
}

#[derive(Debug, Clone, Copy)]
pub enum SyncStatus {
    Syncing {
        target: u64,
        current: u64,
    },
    Synced,
    ///THE ERROR IS BASICALLY A FLIP, 1 FOR NON ERROR, 0 FOR ERROR
    Err(u8),
}

#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    #[error("Invalid block: {0}")]
    InvalidBlock(String),
    #[error("Invalid state root: {0}")]
    InvalidStateRoot(String),
    #[error("Sync error: {0}")]
    SyncError(String),
    #[error("Invalid Signature")]
    InvalidSignature,
    #[error("Invalid response: {0}")]
    Response(String),
    #[error("No consensus")]
    NoConsensus,
    #[error("Parse error")]
    Parse,
}

#[async_trait::async_trait]
pub trait Concensus: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn verify_state_root(&self, block_hash: B256) -> Result<(), Self::Error>;

    // Chain state & finality
    async fn is_finalized(&self, block: BlockHash) -> bool;
    async fn get_finalized_head(&self) -> Result<BlockHash, ConsensusError>;

    // Chain info
    async fn chain_id(&self) -> Result<u64, ConsensusError>;
    async fn genesis_hash(&self) -> B256;
    fn sync_status(&self) -> SyncStatus;

    // Optional: Consensus-specific methods
    fn validators(&self) -> Option<Vec<Address>> {
        None
    }
}

impl ConsensusImpl {
    pub fn new(config: ConsensusConfig, rpc: Arc<RpcClient>) -> Self {
        let state = ConsensusState {
            current_block: config.finalized_block_number,
            finalized_block: config.finalized_block_hash,
            finalized_block_number: config.finalized_block_number,
            sync_status: SyncStatus::Synced,
            min_sync_committee_participants: config.min_sync_comitee,
        };

        Self {
            config,
            state: RwLock::new(state),
            rpc,
            slot_sync: SlotSynchronizer::default(),
        }
    }

    pub async fn is_finalized(&self, block: BlockHash) -> Result<bool, ConsensusError> {
        let state = self
            .state
            .read()
            .map_err(|_| ConsensusError::SyncError("Lock poisoned".into()))?;

        let block_number = self
            .rpc
            .get_block_by_hash(block, false)
            .await
            .map_err(|e| ConsensusError::InvalidBlock(e.to_string()))?
            .unwrap()
            .number;

        //ISSUE
        Ok(block_number <= state.current_block)
    }

    pub async fn optimistic_is_finalized_hash(
        &self,
        block: BlockHash,
    ) -> Result<bool, ConsensusError> {
        let block_data = self
            .rpc
            .get_block_by_hash(block, false)
            .await
            .map_err(|e| ConsensusError::InvalidBlock(e.to_string()))?
            .unwrap();

        // Get current slot
        let (current_slot, _, _) = self
            .slot_sync
            .slot_info()
            .map_err(|_| ConsensusError::SyncError("Slot calculation error".to_string()))?;

        // Calculate block's slot
        let block_slot = self.slot_sync.slot_at_timestamp(
            Utc.timestamp_opt(block_data.timestamp as i64, 0)
                .single()
                .ok_or_else(|| ConsensusError::SyncError("Invalid block timestamp".to_string()))?,
        );

        // A block is considered finalized after 2 epochs
        const EPOCHS_FOR_FINALITY: u64 = 2;
        let slots_needed = EPOCHS_FOR_FINALITY * 32;

        Ok(current_slot >= block_slot + slots_needed)
    }

    pub async fn optimistic_is_finalized_number(&self, block: u64) -> Result<bool, ConsensusError> {
        let block_data = self
            .rpc
            .get_block_header_by_number(block, false)
            .await
            .map_err(|e| ConsensusError::InvalidBlock(e.to_string()))?
            .unwrap();

        // Get current slot
        let (current_slot, _, _) = self
            .slot_sync
            .slot_info()
            .map_err(|_| ConsensusError::SyncError("Slot calculation error".to_string()))?;

        // Calculate block's slot
        let block_slot = self.slot_sync.slot_at_timestamp(
            Utc.timestamp_opt(block_data.timestamp as i64, 0)
                .single()
                .ok_or_else(|| ConsensusError::SyncError("Invalid block timestamp".to_string()))?,
        );

        // A block is considered finalized after 2 epochs
        const EPOCHS_FOR_FINALITY: u64 = 2;
        let slots_needed = EPOCHS_FOR_FINALITY * 32;

        Ok(current_slot >= block_slot + slots_needed)
    }

    pub async fn get_latest_finalized_block_number(&self) -> Result<u64, ConsensusError> {
        let res = self
            .rpc
            .get_block_header_with_tag("finalized", false)
            .await
            .map_err(|_| ConsensusError::SyncError("Failed to get block header".to_string()))?
            .ok_or_else(|| ConsensusError::SyncError("Invalid block timestamp".to_string()))?;

        Ok(res.number)
    }

    pub async fn chain_id(&self) -> Result<u64, ConsensusError> {
        // match self.rpc.get_chain_id().await {
        //     Ok(id) => id
        //         .as_u64()
        //         .ok_or_else(|| ConsensusError::SyncError("Invalid chain ID".into())),
        //     Err(e) => Err(ConsensusError::SyncError(e.to_string())),
        // }
        todo!()
    }

    pub async fn update_state(&self) -> Result<(), ConsensusError> {
        // let latest = self
        //     .rpc
        //     .get_block_number()
        //     .await
        //     .map_err(|e| ConsensusError::SyncError(e.to_string()))?;

        // let mut state = self.state.write().unwrap();

        // state.sync_status = if latest > state.current_block {
        //     SyncStatus::Syncing {
        //         target: latest,
        //         current: state.current_block,
        //     }
        // } else {
        //     SyncStatus::Synced
        // };
        todo!()
    }

    pub async fn update_finalized_head(&self, new_head: BlockHash) -> Result<(), ConsensusError> {
        let mut state = self
            .state
            .write()
            .map_err(|_| ConsensusError::SyncError("Lock poisoned".into()))?;

        let new_block = self
            .rpc
            .get_block_by_hash(new_head, false)
            .await
            .map_err(|e| ConsensusError::InvalidBlock(e.to_string()))?;

        let bind = new_block
            .ok_or_else(|| ConsensusError::SyncError("Invalid chain ID".into()))?
            .number;

        if bind > state.current_block {
            state.finalized_block = new_head;
            state.finalized_block_number = bind;
        }

        Ok(())
    }

    pub async fn verify_finality(&self) -> Result<bool, ConsensusError> {
        let state = self
            .state
            .read()
            .map_err(|_| ConsensusError::SyncError("Lock poisoned".into()))?;

        // Verify finalized chain
        self.optimistic_is_finalized_hash(state.finalized_block)
            .await
    }

    pub fn verify_sync_committee(
        &self,
        _sync_aggregate: &SyncAggregate,
    ) -> Result<(), ConsensusError> {
        unimplemented!()
    }

    pub fn verify_signature(&self, _signatures: &Vec<u8>) -> bool {
        unimplemented!()
    }

    pub fn verify_finality_proof(
        &self,
        _attested_header: &BlockHeader,
        _finalized_header: &BlockHeader,
        _finality_branch: &Vec<B256>,
    ) -> Result<(), ()> {
        todo!()
    }

    pub async fn verify_header(&self, _header: BlockHeader) -> Result<(), ConsensusError> {
        todo!()
    }
}

#[async_trait::async_trait]
impl Concensus for ConsensusImpl {
    type Error = ConsensusError;

    async fn is_finalized(&self, block: BlockHash) -> bool {
        let state = self.state.read().unwrap();
        block <= state.finalized_block
    }

    /// Helper method to get the current finalized head
    async fn get_finalized_head(&self) -> Result<BlockHash, ConsensusError> {
        self.state
            .read()
            .map_err(|_| ConsensusError::SyncError("Lock poisoned".into()))
            .map(|state| state.finalized_block)
    }

    async fn chain_id(&self) -> Result<u64, ConsensusError> {
        // match self.rpc.get_chain_id().await {
        //     Ok(id) => id
        //         .as_u64()
        //         .ok_or_else(|| ConsensusError::SyncError("Invalid chain ID".into())),
        //     Err(e) => Err(ConsensusError::SyncError(e.to_string())),
        // }
        todo!()
    }

    async fn genesis_hash(&self) -> B256 {
        self.config.genesis_hash
    }

    fn sync_status(&self) -> SyncStatus {
        self.state.read().unwrap().sync_status
    }

    fn validators(&self) -> Option<Vec<Address>> {
        None
    }

    async fn verify_state_root(&self, _block_hash: B256) -> Result<(), Self::Error> {
        unimplemented!()
    }
}
