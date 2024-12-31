use crate::{palantiri::rpc::RpcClient, shire::concensus::ConsensusImpl, types::BlockHeader};
use alloy::primitives::BlockHash;
use log::info;
use std::arch::aarch64::{_prefetch, _PREFETCH_LOCALITY0, _PREFETCH_LOCALITY3, _PREFETCH_READ};
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
#[derive(Debug)]
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
        self.sync_block().await?;

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
            let mut old_chain = current_block;
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
            .SyncedState
            .as_ref()
            .ok_or(NodeError::State("SyncedState not initialized".to_string()))?
            .write()
            .await;
        self.NotSyncedState = None;

        let latest = self
            .rpc
            .get_block_number()
            .await
            .map_err(|e| NodeError::Rpc(e.to_string()))?;

        while state.current_block < latest {
            // Sync batch of blocks
            let start = state.current_block + 1;
            let end = (start + 100).min(latest);

            self.sync_block_range(start, end).await?;

            state.current_block = end;
        }
        Ok(())
    }

    pub async fn sync_block(&mut self) -> Result<(), NodeError> {
        let mut state = self
            .SyncedState
            .as_ref()
            .ok_or(NodeError::State("SyncedState not initialized".to_string()))?
            .write()
            .await;
        self.NotSyncedState = None;

        let latest = self
            .rpc
            .get_block_number()
            .await
            .map_err(|e| NodeError::Rpc(e.to_string()))?;

        state.current_block = latest;

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
                    .verify_block(block.parent_hash)
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
        let mut interval = tokio::time::interval(Duration::from_secs(2));
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
            interval.tick().await;

            // Get latest block from chain
            let chain_head = self
                .rpc
                .get_block_number()
                .await
                .map_err(|e| NodeError::Rpc(e.to_string()))?;

            let current = state.current_block;

            // // Prefetch the current block into the L1 cache
            // let current_ptr: *const u64 = &current as *const _;

            // unsafe {
            //     _prefetch::<_PREFETCH_READ, _PREFETCH_LOCALITY0>(current_ptr.cast());
            // }

            // Process new blocks if any
            if chain_head > current {
                info!("Processing block {} {} {}", current, latest, chain_head);

                // let block = self.rpc.get_block_by_number(latest, false)
                //     .await
                //     .map_err(|e| NodeError::Rpc(e.to_string()))?;

                // self.handle_reorg(block).await?;
                state.current_block = chain_head;

                info!("Updated chain head {}", chain_head);
            }
        }
    }

    pub async fn track_finality(&self) -> Result<(), NodeError> {
        let mut interval = tokio::time::interval(Duration::from_secs(12));

        loop {
            interval.tick().await;

            // Get finalized epoch from consensus layer
            let finalized = self
                .consensus
                .get_finalized_number()
                .await
                .map_err(|e| NodeError::Sync(e.to_string()))?;

            let mut state = self
                .SyncedState
                .as_ref()
                .ok_or(NodeError::State("Not synced".into()))?
                .write()
                .await;

            if finalized > state.current_block {
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
    use crate::palantiri::rpc::{RpcRequest, Transport};
    use crate::palantiri::transport::http::TransportBuilder;
    use crate::palantiri::RpcError;
    use crate::shire::concensus::{ConsensusConfig, ConsensusState};
    use crate::types::Block;

    use super::*;
    use alloy::primitives::{B256, U256};
    use async_trait::async_trait;
    use mockall::predicate::*;
    use mockall::*;
    use serde_json::json;
    use tokio::sync::watch;
    use tokio_test::block_on;

    #[automock]
    #[async_trait]
    pub trait RpcClientTrait {
        async fn get_block_by_number(&self, number: u64, full: bool) -> Result<Block, RpcError>;
        async fn get_block_by_hash(&self, hash: BlockHash, full: bool) -> Result<Block, RpcError>;
        async fn get_block_number(&self) -> Result<U256, RpcError>;
    }

    #[async_trait]
    impl RpcClientTrait for RpcClient {
        async fn get_block_by_number(&self, number: u64, full: bool) -> Result<Block, RpcError> {
            let request = RpcRequest {
                jsonrpc: "2.0",
                method: "eth_getBlockByNumber",
                params: json!([format!("0x{:x}", number), full]),
                id: 1,
            };

            self.execute_with_cache(request).await
        }

        async fn get_block_by_hash(&self, hash: BlockHash, full: bool) -> Result<Block, RpcError> {
            let request = RpcRequest {
                jsonrpc: "2.0",
                method: "eth_getBlockByNumber",
                params: json!([format!("0x{:x}", hash), full]),
                id: 1,
            };

            self.execute_with_cache(request).await
        }

        async fn get_block_number(&self) -> Result<U256, RpcError> {
            let number = 64;
            let full_tx = true;
            let request = RpcRequest {
                jsonrpc: "2.0",
                method: "eth_getBlockByNumber",
                params: json!([format!("0x{:x}", number), full_tx]),
                id: 1,
            };

            self.execute_with_cache(request).await
        }
    }

    use tracing_subscriber::{fmt, EnvFilter};

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
            TransportBuilder::new("https://sepolia.base.org".to_string()).build_http(),
        );
        let mock = rpc.get_block_by_number(64, true).await.unwrap();
        println!("{:?}", mock);
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
        let a = node.watch_new_blocks().await.unwrap();
    }
}
