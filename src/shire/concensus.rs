use std::sync::{Arc, RwLock};

use alloy::primitives::{Address, BlockHash, B256, U256};

use crate::palantiri::rpc::RpcClient;

#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    pub chain_id: u64,
    pub finalized_block_number: u64,
    pub genesis_hash: B256,
    pub finalized_block_hash: B256,
    pub sync_period: u64,
}

#[derive(Debug)]
pub struct ConsensusState {
    pub current_block: u64,
    pub finalized_block: BlockHash,
    pub sync_status: SyncStatus,
}

pub struct ConsensusImpl {
    pub config: ConsensusConfig,
    state: RwLock<ConsensusState>,
    rpc: Arc<RpcClient>,
}

#[derive(Debug, Clone, Copy)]
pub enum SyncStatus {
    Syncing { target: u64, current: u64 },
    Synced,
    ///THE ERROR IS BASICALLY A FLIP, 1 FOR NON ERROR, 0 FOR ERROR
    Err(u8)
}

#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    #[error("Invalid block: {0}")]
    InvalidBlock(String),
    #[error("Invalid state root: {0}")]
    InvalidStateRoot(String),
    #[error("Sync error: {0}")]
    SyncError(String),
}

#[async_trait::async_trait]
pub trait Concensus: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn verify_block(&self, block: B256) -> Result<(), Self::Error>;
    async fn verify_state_root(&self, block_hash: B256) -> Result<(), Self::Error>;

    // Chain state & finality
    async fn is_finalized(&self, block: BlockHash) -> bool;
    async fn get_finalized_head(&self) -> BlockHash;

    // Chain info
    async fn chain_id(&self) -> u64;
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
            sync_status: SyncStatus::Synced,
        };

        Self {
            config,
            state: RwLock::new(state),
            rpc,
        }
    }

    pub async fn update_state(&self) -> Result<(), ConsensusError> {
        let latest = self
            .rpc
            .get_block_number()
            .await
            .map_err(|e| ConsensusError::SyncError(e.to_string()))?
            .as_u64()
            .unwrap_or(panic!());

        let mut state = self.state.write().unwrap();

        state.sync_status = if latest > state.current_block {
            SyncStatus::Syncing {
                target: latest,
                current: state.current_block,
            }
        } else {
            SyncStatus::Synced
        };
        Ok(())
    }
}

#[async_trait::async_trait]
impl Concensus for ConsensusImpl {
    type Error = ConsensusError;

    async fn verify_block(&self, block: B256) -> Result<(), Self::Error> {
        let block = self
            .rpc
            .get_block_by_hash(block, false)
            .await
            .map_err(|e| ConsensusError::InvalidBlock(e.to_string()))?;

        // if !self.is_valid_parent(block.header.parent_hash).await {
        //     return Err(ConsensusError::InvalidBlock("Invalid parent hash".into()));
        // }

        Ok(())
    }

    async fn verify_state_root(&self, block_hash: B256) -> Result<(), Self::Error> {
        let block = self
            .rpc
            .get_block_by_hash(block_hash, false)
            .await
            .map_err(|e| ConsensusError::InvalidBlock(e.to_string()))?;

        if block.header.state_root.is_zero() {
            return Err(ConsensusError::InvalidStateRoot("Empty state root".into()));
        }

        Ok(())
    }

    async fn is_finalized(&self, block: BlockHash) -> bool {
        let state = self.state.read().unwrap();
        //ISSUE
        block <= state.finalized_block
    }

    async fn get_finalized_head(&self) -> BlockHash {
        self.state.read().unwrap().finalized_block
    }

    async fn chain_id(&self) -> u64 {
        //ISSUE: ERROR HANDLING
        self.rpc.get_chain_id().await.unwrap().as_u64().unwrap()
        
    }

    async fn genesis_hash(&self) -> B256 {
        // Implementation...
        todo!()
    }

    fn sync_status(&self) -> SyncStatus {
        self.state.read().unwrap().sync_status
    }

    fn validators(&self) -> Option<Vec<Address>> {
        None
    }
}
