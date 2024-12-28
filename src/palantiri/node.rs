use crate::{palantiri::rpc::RpcClient, shire::concensus::ConsensusImpl, types::BlockHeader};
use alloy::primitives::BlockHash;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};
use tokio::time::Duration;

#[derive(Debug, Clone)]
pub enum ChainEvent {
    NewBlock(BlockHeader),
    Reorg { old_tip: u64, common_ancestor: u64 },
    Finalized(u64),
}

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("Sync error: {0}")]
    Sync(String),
    #[error("State error: {0}")]
    State(String),
    #[error("RPC error: {0}")]
    Rpc(String),
}

#[derive(Debug)]
pub struct SyncedNodeState {
    current_block: u64,
    finalized_block: u64,
    peers: Vec<String>,
}

#[derive(Debug)]
pub struct NotSyncedNodeState {
    current_block: u64,
    finalized_block: u64,
    peers: Vec<String>,
}

#[allow(non_snake_case)]
pub struct Node {
    consensus: Arc<ConsensusImpl>,
    rpc: Arc<RpcClient>,
    SyncedState: Option<RwLock<SyncedNodeState>>,
    NotSyncedState: Option<RwLock<NotSyncedNodeState>>,
    event_tx: broadcast::Sender<ChainEvent>,
}

impl Node {
    pub fn new(consensus: Arc<ConsensusImpl>, rpc: Arc<RpcClient>) -> Self {
        let (event_tx, _) = broadcast::channel(100);

        Self {
            consensus,
            rpc,
            SyncedState: Some(RwLock::new(SyncedNodeState {
                current_block: 0,
                finalized_block: 0,
                peers: Vec::new(),
            })),
            NotSyncedState: None,
            event_tx,
        }
    }

    pub async fn start(&mut self) -> Result<(), NodeError> {
        // Initialize state
        self.initialize().await?;

        // Start sync pipeline
        self.sync_blocks().await?;

        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ChainEvent> {
        self.event_tx.subscribe()
    }

    async fn handle_reorg(&self, new_block: BlockHeader) -> Result<(), NodeError> {
        let state = self
            .SyncedState
            .as_ref()
            .ok_or(NodeError::State("Not in synced state".into()))?
            .read()
            .await;

        let current_block = self
            .rpc
            .get_block_by_number(state.current_block, false)
            .await
            .map_err(|e| NodeError::Rpc(e.to_string()))?;

        // Check if reorg needed
        if new_block.number <= state.current_block {
            // Get parent blocks until common ancestor found
            let mut old_chain = current_block.header;
            let hash = new_block.parent_hash;
            let mut new_chain = new_block;

            while old_chain.parent_hash != new_chain.parent_hash {
                old_chain = self
                    .rpc
                    .get_block_by_hash(old_chain.parent_hash, false)
                    .await
                    .map_err(|e| NodeError::Rpc(e.to_string()))?
                    .header;

                new_chain = self
                    .rpc
                    .get_block_by_hash(new_chain.parent_hash, false)
                    .await
                    .map_err(|e| NodeError::Rpc(e.to_string()))?
                    .header;
            }

            // Get common ancestor
            let ancestor = self
                .rpc
                .get_block_by_hash(old_chain.parent_hash, false)
                .await
                .map_err(|e| NodeError::Rpc(e.to_string()))?
                .header;

            // Emit reorg event
            self.event_tx
                .send(ChainEvent::Reorg {
                    old_tip: old_chain.number,
                    common_ancestor: ancestor.number,
                })
                .map_err(|e| NodeError::State(e.to_string()))?;
        }

        Ok(())
    }

    async fn initialize(&self) -> Result<(), NodeError> {
        let genesis = self
            .rpc
            .get_block_by_number(0, false)
            .await
            .map_err(|e| NodeError::Rpc(e.to_string()))?;

        let mut state = self
            .SyncedState
            .as_ref()
            .ok_or(NodeError::State("SyncedState not initialized".to_string()))?
            .write()
            .await;
        state.current_block = 0;
        state.finalized_block = 0;

        Ok(())
    }

    pub async fn sync_blocks(&mut self) -> Result<(), NodeError> {
        let mut state = self
            .NotSyncedState
            .as_ref()
            .ok_or(NodeError::State("SyncedState not initialized".to_string()))?
            .write()
            .await;
        self.SyncedState = None;

        let latest = self
            .rpc
            .get_block_number()
            .await
            .map_err(|e| NodeError::Rpc(e.to_string()))?
            .as_u64()
            .unwrap();

        while state.current_block < latest {
            // Sync batch of blocks
            let start = state.current_block + 1;
            let end = (start + 100).min(latest);

            self.sync_block_range(start, end).await?;

            state.current_block = end;
        }
        Ok(())
    }

    async fn sync_block_range(&self, mut start: u64, end: u64) -> Result<(), NodeError> {
        while start <= end {
            for number in start..=end {
                let block = self
                    .rpc
                    .get_block_by_number(number, false)
                    .await
                    .map_err(|e| NodeError::Rpc(e.to_string()))?;

                // Verify block header
                self.consensus
                    .verify_block(block.header.parent_hash)
                    .await
                    .map_err(|e| NodeError::Sync(e.to_string()))?;

                // Update state
                let mut state = self
                    .SyncedState
                    .as_ref()
                    .ok_or(NodeError::State("SyncedState not initialized".to_string()))?
                    .write()
                    .await;
                state.current_block = number;
            }

            start += 1;
        }
        Ok(())
    }

    pub async fn watch_new_blocks(&self) -> Result<(), NodeError> {
        let mut interval = tokio::time::interval(Duration::from_secs(12));
        
        loop {
            interval.tick().await;
            
            // Get latest block number
            let latest = self.rpc.get_block_number()
                .await
                .map_err(|e| NodeError::Rpc(e.to_string()))?
                .as_u64()
                .unwrap();
    
            // Get current state
            let mut state = self.SyncedState
                .as_ref()
                .ok_or(NodeError::State("Not synced".into()))?
                .write()
                .await;
    
            // Process new blocks if any
            if latest > state.current_block {
                let block = self.rpc.get_block_by_number(latest, true)
                    .await
                    .map_err(|e| NodeError::Rpc(e.to_string()))?;
    
                self.handle_reorg(block.header).await?;
                state.current_block = latest;
            }
        }
    }

    pub async fn track_finality(&self) -> Result<(), NodeError> {
        let mut interval = tokio::time::interval(Duration::from_secs(12));
        
        loop {
            interval.tick().await;
            
            // Get finalized epoch from consensus layer
            let finalized = self.consensus.get_finalized_number()
                .await
                .map_err(|e| NodeError::Sync(e.to_string()))?;

            let mut state = self.SyncedState
                .as_ref()
                .ok_or(NodeError::State("Not synced".into()))?
                .write()
                .await;

            if finalized > state.current_block {
                // Update finalized block
                state.finalized_block = finalized;
                
                // Emit finalized event
                self.event_tx.send(ChainEvent::Finalized(finalized))
                    .map_err(|e| NodeError::State(e.to_string()))?;
            }
        }
    }
}
