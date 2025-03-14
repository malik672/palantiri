use crate::rpc::RpcClient;
use alloy_primitives::{keccak256, FixedBytes, B256, U64};
use log::info;
use mordor::SlotSynchronizer;
use parser::types::{Block, BlockHeader};
use rlp::encode::encode_list;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};
use tokio::time::Duration;

const BROADCAST_CHANNEL_SIZE: usize = 1000;

#[derive(Debug, Clone)]
pub enum ChainEvent {
    NewBlock(BlockHeader),
    Reorg { old_tip: u64, common_ancestor: u64 },
    Finalized(U64),
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
    pub current_block: U64,
    pub finalized_block: U64,
}

#[allow(non_snake_case)]
#[derive(Debug)]
pub struct Node {
    // pub consensus: Arc<ConsensusImpl>,
    pub rpc: Arc<RpcClient>,
    pub SyncedState: Option<RwLock<SyncedNodeState>>,
    pub event_tx: broadcast::Sender<ChainEvent>,
}

impl Node {
    pub fn new(rpc: Arc<RpcClient>) -> Self {
        let (event_tx, _) = broadcast::channel(BROADCAST_CHANNEL_SIZE);

        Self {
            rpc,
            SyncedState: Some(RwLock::new(SyncedNodeState {
                current_block: U64::from(0),
                finalized_block: U64::from(0),
            })),
            event_tx,
        }
    }

    pub async fn start(&mut self) -> Result<(), NodeError> {
        // Start sync pipeline
        self.watch_new_blocks().await?;

        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ChainEvent> {
        self.event_tx.subscribe()
    }

    /// ISSUE: This function is not yet implemented correctly
    pub async fn sync_block_range_head(
        &self,
        start: u64,
        end: u64,
        batch_size: u64,
        max_retries: u32,
    ) -> Result<(), NodeError> {
        for batch_start in (start..end).step_by(batch_size as usize) {
            let batch_end = (batch_start + batch_size).min(end);

            let futures: Vec<_> = (batch_start..batch_end)
            .map(|block_num| async move {
                let mut attempt = 0;
                loop {
                    match self.rpc.get_block_header_by_number(block_num, false).await {
                        Ok(Some(block)) => return Ok(block),
                        Ok(None) => return Ok(BlockHeader::default()),
                        Err(e) => {
                            attempt += 1;
                            if attempt >= max_retries {
                                return Err(NodeError::Rpc(e.to_string()));
                            }
                            let delay = 1000 * 2u64.pow(attempt - 1);
                            info!(
                                "Failed to fetch block {}, attempt {}/{}. Retrying in {}ms. Error: {}", 
                                block_num, attempt, max_retries, delay, e
                            );
                            tokio::time::sleep(Duration::from_millis(delay)).await;
                            continue;
                        }
                    }
                }
            })
            .collect();

            let _blocks = futures::future::join_all(futures)
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;
        }
        let mut state = self
            .SyncedState
            .as_ref()
            .ok_or(NodeError::State("SyncedState not initialized".to_string()))?
            .write()
            .await;

        state.current_block = U64::from(end);
        Ok(())
    }

    /// ISSUE: This function is not yet implemented correctly
    pub async fn sync_block_range(&self, start: u64, end: u64) -> Result<(), NodeError> {
        const BATCH_SIZE: u64 = 1000;
        const MAX_RETRIES: u32 = 5;
        for batch_start in (start..end).step_by(BATCH_SIZE as usize) {
            let batch_end = (batch_start + BATCH_SIZE).min(end);

            let futures: Vec<_> = (batch_start..batch_end)
            .map(|block_num| async move {
                let mut attempt = 0;
                loop {
                    match self.rpc.get_block_by_number(block_num, false).await {
                        Ok(Some(block)) => return Ok(block),
                        Ok(None) => return Ok(Block::default()),
                        Err(e) => {
                            attempt += 1;
                            if attempt >= MAX_RETRIES {
                                return Err(NodeError::Rpc(e.to_string()));
                            }
                            let delay = 1000 * 2u64.pow(attempt - 1);
                            info!(
                                "Failed to fetch block {}, attempt {}/{}. Retrying in {}ms. Error: {}", 
                                block_num, attempt, MAX_RETRIES, delay, e
                            );
                            println!("Failed to fetch block {}, attempt {}/{}. Retrying in {}ms. Error: {}", 
                                block_num, attempt, MAX_RETRIES, delay, e
                            );
                            tokio::time::sleep(Duration::from_millis(delay)).await;
                            continue;
                        }
                    }
                }
            })
            .collect();

            let _blocks = futures::future::join_all(futures)
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;
        }
        // let mut state = self
        //     .SyncedState
        //     .as_ref()
        //     .ok_or(NodeError::State("SyncedState not initialized".to_string()))?
        //     .write()
        //     .await;

        // state.current_block = end;
        Ok(())
    }

    /// Watches for new blocks by synchronizing with Ethereum slot timings.
    ///
    /// This function implements an optimistic block tracking strategy to minimize RPC calls
    /// while maintaining accurate chain head tracking. It uses the mathematical relationship  
    /// between slot numbers and block numbers (difference) for validation.
    ///
    ///
    /// # Arguments
    /// - &self: Reference to Node instance
    ///
    /// # Returns
    /// - Result<(), NodeError>: Ok(()) on success, NodeError on failure
    ///
    /// # Example
    /// ```no_run
    /// let node = Node::new(consensus, rpc);
    /// node.watch_new_blocks().await?;
    /// ```
    pub async fn watch_new_blocks(&self) -> Result<(), NodeError> {
        let time = SlotSynchronizer::default();
        let mut block_slot_difference: U64 = U64::from(10787043);

        // Get latest block number
        let mut latest = self
            .rpc
            .get_block_number()
            .await
            .map_err(|e| NodeError::Rpc(e.to_string()))?;

        info!("Starting block watcher with latest block: {}", latest);

        // Get current state
        let mut state = self
            .SyncedState
            .as_ref()
            .ok_or(NodeError::State("Not synced".into()))?
            .write()
            .await;

        loop {
            // Get current slot info
            let (slot, _elapsed, remaining) = time.slot_info().unwrap();

            // Use remaining time to determine next check
            let wait_time = Duration::from_secs(remaining as u64);
            tokio::time::sleep(wait_time).await;

            let current = state.current_block;

            // Process new blocks if any
            if latest > current {
                info!("Processing block {} {}", current, latest);

                if (latest - U64::from(slot)) == U64::from(block_slot_difference) {
                    state.current_block = latest;
                } else {
                    latest = self
                        .rpc
                        .get_block_number()
                        .await
                        .map_err(|e| NodeError::Rpc(e.to_string()))?;
                    state.current_block = latest;
                    block_slot_difference = latest - U64::from(slot);
                }
                info!("Updated chain head {}", latest);
                latest += U64::from(1);
            }
        }
    }

    pub async fn compute_hash(
        block_number: U64,
        block_hash: B256,
        tx_root: B256,
        state_root: B256,
        receipts_root: B256,
    ) -> FixedBytes<32> {
        let mut out: Vec<u8> = Vec::new();

        let number_bytes = block_number.to_be_bytes_vec();

        let fields: [&[u8]; 5] = [
            &number_bytes,
            block_hash.0.as_slice(),
            tx_root.0.as_slice(),
            state_root.0.as_slice(),
            receipts_root.0.as_slice(),
        ];

        encode_list::<&[u8], [u8]>(&fields, &mut out);
        keccak256(out)
    }

    pub async fn track_finality(&self) -> Result<(), NodeError> {
        let mut interval = tokio::time::interval(Duration::from_secs(12));

        loop {
            interval.tick().await;

            // Get finalized epoch from consensus layer
            let finalized = self
                .rpc
                .get_block_number()
                .await
                .map_err(|e| NodeError::Sync(e.to_string()))?;

            let mut state = self
                .SyncedState
                .as_ref()
                .ok_or(NodeError::State("Not synced".into()))?
                .write()
                .await;

            if U64::from(finalized) > state.finalized_block {
                // Update finalized block
                state.finalized_block = U64::from(finalized);

                // Emit finalized event
                self.event_tx
                    .send(ChainEvent::Finalized(finalized))
                    .map_err(|e| NodeError::State(e.to_string()))?;
            }
        }
    }
}
