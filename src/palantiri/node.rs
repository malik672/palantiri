use crate::{
    palantiri::rpc::RpcClient,
    shire::concensus::ConsensusImpl,
    types::{BlockHeader, NUM_HASH_DATA},
};
use log::info;
use mordor::SlotSynchronizer;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};
use tokio::time::Duration;

const BROADCAST_CHANNEL_SIZE: usize = 1000;

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
    pub current_block: u64,
    pub finalized_block: u64,
}

#[allow(non_snake_case)]
#[derive(Debug)]
pub struct Node {
    pub consensus: Arc<ConsensusImpl>,
    pub rpc: Arc<RpcClient>,
    pub SyncedState: Option<RwLock<SyncedNodeState>>,
    pub event_tx: broadcast::Sender<ChainEvent>,
}

impl Node {
    pub fn new(consensus: Arc<ConsensusImpl>, rpc: Arc<RpcClient>) -> Self {
        let (event_tx, _) = broadcast::channel(BROADCAST_CHANNEL_SIZE);

        Self {
            consensus,
            rpc,
            SyncedState: Some(RwLock::new(SyncedNodeState {
                current_block: 0,
                finalized_block: 0,
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
    pub async fn sync_block_range(&self, start: u64, end: u64) -> Result<(), NodeError> {
        const BATCH_SIZE: u64 = 10000;
        const MAX_RETRIES: u32 = 5;
        for batch_start in (start..end).step_by(BATCH_SIZE as usize) {
            let batch_end = (batch_start + BATCH_SIZE).min(end);

            let futures: Vec<_> = (batch_start..batch_end)
            .map(|block_num| async move {
                let mut attempt = 0;  
                loop {
                    match self.rpc.get_block_header_by_number(block_num, false).await {
                        Ok(Some(block)) => return Ok(block),
                        Ok(None) => return Ok(BlockHeader::default()),
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
        let mut state = self
            .SyncedState
            .as_ref()
            .ok_or(NodeError::State("SyncedState not initialized".to_string()))?
            .write()
            .await;

        state.current_block = end;
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
        let mut block_slot_difference: u64 = 10787043;

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

                if (latest - slot) == block_slot_difference {
                    state.current_block = latest;
                } else {
                    latest = self
                        .rpc
                        .get_block_number()
                        .await
                        .map_err(|e| NodeError::Rpc(e.to_string()))?;
                    state.current_block = latest;
                    block_slot_difference = latest - slot;
                    println!("Block slot difference updated to {}", latest);
                }
                info!("Updated chain head {}", latest);
                latest += 1;
            }
        }
    }

    pub async fn compute_hash() {

    }

    ///ISSUE: This function is not yet implemented correctly
    pub async fn track_finality(&self) -> Result<(), NodeError> {
        let mut interval = tokio::time::interval(Duration::from_secs(12));

        loop {
            interval.tick().await;

            // Get finalized epoch from consensus layer
            let finalized = self
                .consensus
                .get_latest_finalized_block_number()
                .await
                .map_err(|e| NodeError::Sync(e.to_string()))?;

            let mut state = self
                .SyncedState
                .as_ref()
                .ok_or(NodeError::State("Not synced".into()))?
                .write()
                .await;

            if finalized > state.finalized_block {
                // Update finalized block
                state.finalized_block = finalized;

                // Emit finalized event
                self.event_tx
                    .send(ChainEvent::Finalized(finalized))
                    .map_err(|e| NodeError::State(e.to_string()))?;
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::palantiri::transport::http::TransportBuilder;
    use crate::shire::concensus::ConsensusConfig;

    use super::*;
    use alloy::primitives::B256;

    use tracing_subscriber::EnvFilter;

    fn setup_logging() {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env()
                    .add_directive(tracing::Level::INFO.into())
                    .add_directive("light_client=debug".parse().unwrap()),
            )
            .with_test_writer()
            .init();
    }

    #[tokio::test]
    async fn test_rpc_client() {
        let rpc = RpcClient::new(
            TransportBuilder::new(
                "https://eth-mainnet.g.alchemy.com/v2/4yEoD1kdx0Eocdx_HFeGAOPsbysH3yRM".to_string(),
            )
            .build_http(),
        );
        let mock = rpc
            .get_block_header_by_number(215466241, false)
            .await
            .map_err(|e| NodeError::Rpc(e.to_string()))
            .unwrap();
        println!("{:?}", mock.unwrap().sync_aggregate);
    }

    #[tokio::test]
    async fn test_node() {
        let rpc = RpcClient::new(
            TransportBuilder::new("https://sepolia.base.org".to_string()).build_http(),
        );
        let mut node = Node::new(
            Arc::new(ConsensusImpl::new(
                ConsensusConfig {
                    chain_id: 1,
                    finalized_block_number: 0,
                    genesis_hash: B256::default(),
                    finalized_block_hash: B256::default(),
                    sync_period: 10,
                    min_sync_comitee: 30,
                },
                Arc::new(rpc.clone()),
            )),
            Arc::new(rpc),
        );
        node.start().await.unwrap();

        //Test is state is already synced
        assert!(!node.SyncedState.is_none());
        let binding = node.SyncedState.unwrap();
        let _node = binding.read().await;
        assert!(_node.current_block > 0);

        //At this point, the node is already synced but it's not finalized yet
        assert!(_node.finalized_block == 0);
    }

    #[tokio::test]
    async fn test_watch_new_blocks() {
        setup_logging();
        let rpc = RpcClient::new(
            TransportBuilder::new(
                "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4".to_string(),
            )
            .build_http(),
        );
        let node = Node::new(
            Arc::new(ConsensusImpl::new(
                ConsensusConfig {
                    chain_id: 1,
                    finalized_block_number: 0,
                    genesis_hash: B256::default(),
                    finalized_block_hash: B256::default(),
                    sync_period: 10,
                    min_sync_comitee: 30,
                },
                Arc::new(rpc.clone()),
            )),
            Arc::new(rpc),
        );

        // the watch new blocks checks for new blocks
        let _a = node.watch_new_blocks().await.unwrap();
    }

    #[tokio::test]
    async fn sync_blocks() {
        setup_logging();

        let rpc = RpcClient::new(
            TransportBuilder::new(
                "https://mainnet.infura.io/v3/de690e56c52741b5a18be8c49c2f2b01".to_string(),
            )
            .build_http(),
        );
        let mut node = Node::new(
            Arc::new(ConsensusImpl::new(
                ConsensusConfig {
                    chain_id: 1,
                    finalized_block_number: 0,
                    genesis_hash: B256::default(),
                    finalized_block_hash: B256::default(),
                    sync_period: 10,
                    min_sync_comitee: 30,
                },
                Arc::new(rpc.clone()),
            )),
            Arc::new(rpc),
        );
    }
}
