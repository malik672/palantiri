use std::time::Duration;

use alloy_primitives::B256;
use mordor::{SlotSynchronizer, SLOTS_PER_PERIOD};
use parser::types::{
    Beacon, FinalityUpdate, LightClientBootstrap, LightOptimisticUpdate, SyncCommittee, Updates,
};
use reqwest::Client;

use crate::concensus::ConsensusError;

/// Stores the current state of a light client, including finalized and optimistic headers,
/// and sync committee information

#[derive(Debug, Default, Clone)]
pub struct LightClientStore {
    pub finalized_header: Beacon,
    pub optimistic_header: Beacon,
    pub attested_header: Beacon,
    pub current_sync_committee: SyncCommittee,
    pub next_sync_committee: Option<SyncCommittee>,
}

/// Manages the synchronization process for a light client by coordinating updates
/// and maintaining the client state

#[derive(Debug, Default, Clone)]
pub struct LightClientSyncer {
    pub client: LightClient,
    pub slot_sync: SlotSynchronizer,
    pub store: Option<LightClientStore>,
}

/// A light client implementation for interacting with Ethereum beacon chain endpoints.
/// This client supports concurrent querying of multiple endpoints for redundancy and
/// consensus verification.

#[derive(Debug, Default, Clone)]
pub struct LightClient {
    pub endpoints: Vec<String>,
    pub client: Client,
}

impl LightClientSyncer {
    pub fn new(endpoints: Vec<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        Self {
            client: LightClient { endpoints, client },
            slot_sync: SlotSynchronizer::default(),
            store: None,
        }
    }

    /// Retrieves the latest finality update from the beacon chain.
    ///
    /// Queries all configured endpoints concurrently and selects the popular response
    ///
    /// # Returns
    /// - `Result<FinalityUpdate, ConsensusError>`: The latest finality update or error
    pub async fn get_latest_finality_update(&self) -> Result<FinalityUpdate, ConsensusError> {
        let mut responses = Vec::new();

        // Query all endpoints concurrently
        let results = futures::future::join_all(self.client.endpoints.iter().map(|endpoint| {
            self.client
                .client
                .get(format!(
                    "{}/eth/v1/beacon/light_client/finality_update",
                    endpoint
                ))
                .send()
        }))
        .await;

        // Collect responses with signatures
        for response in results {
            if let Ok(resp) = response {
                let input = resp
                    .bytes()
                    .await
                    .map(|b| b.to_vec())
                    .map_err(|_| ConsensusError::Parse)?;

                let update = FinalityUpdate::parse(&input).ok_or(ConsensusError::Parse)?;

                responses.push((update.sync_aggregate.sync_committee_signature, update));
            }
        }

        responses.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(responses[0].1.clone())
    }

    /// Retrieves light client updates for a specific period.
    ///
    /// # Arguments
    /// * `period` - The sync committee period to query
    /// * `count` - Number of updates to retrieve
    ///
    /// # Returns
    /// - `Result<Updates, ConsensusError>`: The requested updates or error
    pub async fn get_latest_update(
        &self,
        period: u64,
        count: u64,
    ) -> Result<Updates, ConsensusError> {
        let mut responses = Vec::new();

        // Query all endpoints concurrently
        let results = futures::future::join_all(self.client.endpoints.iter().map(|endpoint| {
            self.client
                .client
                .get(format!(
                    "{}/eth/v1/beacon/light_client/updates?period={}&count={}",
                    endpoint, period, count
                ))
                .send()
        }))
        .await;

        // Collect responses with signatures
        for response in results {
            if let Ok(resp) = response {
                let input = resp
                    .bytes()
                    .await
                    .map(|b| b.to_vec())
                    .map_err(|_| ConsensusError::Parse)?;

                let update = Updates::parse(&input).ok_or(ConsensusError::Parse)?;

                responses.push((update.sync_aggregate.sync_committee_signature, update));
            }
        }

        responses.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(responses[0].1.clone())
    }

    /// Retrieves the light client bootstrap data for a specific block.
    ///
    /// # Arguments
    /// * `block_root` - The root hash of the block to bootstrap from
    ///
    /// # Returns
    /// - `Result<LightClientBootstrap, ConsensusError>`: Bootstrap data or error
    pub async fn get_bootstrap(
        &self,
        block_root: B256,
    ) -> Result<LightClientBootstrap, ConsensusError> {
        let mut responses = Vec::new();

        // Query all endpoints concurrently
        let results = futures::future::join_all(self.client.endpoints.iter().map(|endpoint| {
            self.client
                .client
                .get(format!(
                    "{}/eth/v1/beacon/light_client/bootstrap/{}",
                    endpoint, block_root,
                ))
                .send()
        }))
        .await;

        // Collect responses with signatures
        for response in results {
            if let Ok(resp) = response {
                let input = resp
                    .bytes()
                    .await
                    .map(|b| b.to_vec())
                    .map_err(|_| ConsensusError::Parse)?;

                let update = LightClientBootstrap::parse(&input).ok_or(ConsensusError::Parse)?;

                responses.push((update.current_sync_committee.aggregate_pubkey, update));
            }
        }

        responses.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(responses[0].1.clone())
    }

    /// SIGNICANT ISSUE: WRONG RETURN TYPE
    /// Retrieves the latest optimistic update from the beacon chain.
    ///
    /// Similar to finality update but for optimistic sync data.
    ///
    /// # Returns
    /// - `Result<LightOptimisticUpdate, ConsensusError>`: Latest optimistic update or error
    pub async fn get_optimistic_update(&self) -> Result<LightOptimisticUpdate, ConsensusError> {
        let mut responses = Vec::new();

        // Query all endpoints concurrently
        let results = futures::future::join_all(self.client.endpoints.iter().map(|endpoint| {
            self.client
                .client
                .get(format!(
                    "{}/eth/v1/beacon/light_client/optimistic_update",
                    endpoint,
                ))
                .send()
        }))
        .await;

        // Collect responses with signatures
        for response in results {
            if let Ok(resp) = response {
                let input = resp
                    .bytes()
                    .await
                    .map(|b| b.to_vec())
                    .map_err(|_| ConsensusError::Parse)?;

                let update = LightOptimisticUpdate::parse(&input).ok_or(ConsensusError::Parse)?;

                responses.push((update.sync_aggregate.sync_committee_signature, update));
            }
        }

        responses.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(responses[0].1.clone())
    }

    pub fn get_sync_committee_period(&self, slot: u64) -> u64 {
        slot / SLOTS_PER_PERIOD
    }

    pub fn is_next_sync_committee_known(&self) -> bool {
        self.store
            .as_ref()
            .map(|store| store.next_sync_committee.is_some())
            .unwrap_or(false)
    }

    /// Initialize a new light client store from bootstrap data
    /// ISSUE: since it's the same header why not combine it instead of clone
    pub async fn initialize_store(
        &mut self,
        trusted_block_root: B256,
    ) -> Result<(), ConsensusError> {
        let bootstrap = self.get_bootstrap(trusted_block_root).await?;
        self.store = Some(LightClientStore {
            finalized_header: bootstrap.header.beacon.clone(),
            optimistic_header: bootstrap.header.beacon.clone(),
            attested_header: bootstrap.header.beacon,
            current_sync_committee: bootstrap.current_sync_committee,
            next_sync_committee: None,
        });

        Ok(())
    }

    /// Main sync loop that keeps the light client in sync with the network
    pub async fn sync(&mut self, trusted_block_root: B256) -> Result<(), ConsensusError> {
        // Initialize store if needed
        if self.store.is_none() {
            self.initialize_store(trusted_block_root).await?;
        }
        let mut store = self.store.clone().ok_or(ConsensusError::Parse)?;

        loop {
            //                 apply_light_client_update              //
            let current_slot = self
                .slot_sync
                .current_slot()
                .map_err(|_| ConsensusError::Parse)?;
            let finalized_period = self.get_sync_committee_period(store.finalized_header.slot.to());
            let optimistic_period =
                self.get_sync_committee_period(store.optimistic_header.slot.to());
            let current_period = self.get_sync_committee_period(current_slot);

            if finalized_period == optimistic_period && !self.is_next_sync_committee_known() {
                let updates = self.get_latest_update(finalized_period, 1).await?;
                store.next_sync_committee = Some(updates.next_sync_committee);
            }

            if finalized_period + 1 < current_period {
                for period in (finalized_period + 1)..current_period {
                    let updates = self.get_latest_update(period, 1).await?;

                    // Apply the update to move forward
                    store.finalized_header = updates.finalized_header;
                    store.optimistic_header = updates.attested_header;
                    store.current_sync_committee = updates.next_sync_committee;
                    store.next_sync_committee = None;
                }
            }

            // Case 3: Stay up to date with latest updates
            if finalized_period + 1 >= current_period {
                // Process finality update
                if let Ok(finality_update) = self.get_latest_finality_update().await {
                    store.finalized_header = finality_update.finalized_header;
                    store.optimistic_header = finality_update.attested_header;
                }

                // Process optimistic update
                if let Ok(optimistic_update) = self.get_optimistic_update().await {
                    store.optimistic_header = optimistic_update.attested_header;
                }
            }

            let wait_time = self
                .slot_sync
                .time_until_next_slot()
                .map_err(|_| ConsensusError::Parse)?;
            tokio::time::sleep(wait_time).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync() {
        let a = LightClientSyncer::new(vec!["https://eth-beacon-chain.drpc.org/rest/".to_string()]);
        // println!("{:?}", a.get_latest_finality_update().await);
        // println!("{:?}", a.get_latest_update(0, 10).await);
        println!("{:?}", a.get_optimistic_update().await);
    }
}
