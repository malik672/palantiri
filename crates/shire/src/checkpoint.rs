use alloy::primitives::B256;
use eyre::Error;
use reqwest::{Client, ClientBuilder};
use retri::{retry, BackoffSettings};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

use crate::Network;

/// The location where the list of checkpoint services are stored.
pub const CHECKPOINT_SYNC_SERVICES_LIST: &str = "https://raw.githubusercontent.com/ethpandaops/checkpoint-sync-health-checks/master/_data/endpoints.yaml";

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawSlotResponse {
    pub data: RawSlotResponseData,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawSlotResponseData {
    pub slots: Vec<Slot>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    pub slot: u64,
    pub block_root: Option<B256>,
    pub state_root: Option<B256>,
    pub epoch: u64,
    pub time: StartEndTime,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartEndTime {
    /// An ISO 8601 formatted UTC timestamp.
    pub start_time: String,
    /// An ISO 8601 formatted UTC timestamp.
    pub end_time: String,
}

/// A health check for the checkpoint sync service.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    /// If the node is healthy.
    pub result: bool,
    /// An [ISO 8601](https://en.wikipedia.org/wiki/ISO_8601) UTC timestamp.
    pub date: String,
}

/// A checkpoint fallback service.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointFallbackService {
    /// The endpoint for the checkpoint sync service.
    pub endpoint: String,
    /// The checkpoint sync service name.
    pub name: String,
    /// The service state.
    pub state: bool,
    /// If the service is verified.
    pub verification: bool,
    /// Contact information for the service maintainers.
    pub contacts: Option<serde_yaml::Value>,
    /// Service Notes
    pub notes: Option<serde_yaml::Value>,
    /// The service health check.
    pub health: Vec<Health>,
}

/// The CheckpointFallback manages checkpoint fallback services.
#[derive(Debug, Default, Clone, Serialize)]
pub struct CheckpointFallback {
    /// Services Map
    /// Three object present Maiinet: 0, Holesky: 1, Sepolia: 2
    pub services: Vec<(Network, Vec<CheckpointFallbackService>)>,
    /// A list of supported networks to build.
    /// Default: [Mainnet, Holesky, Sepolia]
    pub networks: Vec<Network>,
}

async fn get(req: &str) -> Result<reqwest::Response, Error> {
    retry(
        || async {
            #[cfg(not(target_arch = "wasm32"))]
            let client = ClientBuilder::new()
                .timeout(Duration::from_secs(1))
                .build()
                .unwrap();

            #[cfg(target_arch = "wasm32")]
            let client = ClientBuilder::new().build().unwrap();

            Ok::<_, eyre::Report>(client.get(req).send().await?)
        },
        BackoffSettings::default(),
    )
    .await
}

impl CheckpointFallback {
    /// Create a new instance with default networks
    pub fn new() -> Self {
        Self {
            services: Vec::with_capacity(3),
            networks: vec![Network::Mainnet, Network::Holesky, Network::Sepolia],
        }
    }

    /// Constructs the checkpoint fallback service url for fetching a slot.
    ///
    /// This is an associated function and can be used like so:
    ///
    /// ```rust
    /// use helios_ethereum::config::checkpoints::CheckpointFallback;
    ///
    /// let url = CheckpointFallback::construct_url("https://sync-mainnet.beaconcha.in");
    /// assert_eq!("https://sync-mainnet.beaconcha.in/checkpointz/v1/beacon/slots", url);
    /// ```
    pub fn construct_url(endpoint: &str) -> String {
        format!("{endpoint}/checkpointz/v1/beacon/slots")
    }

    /// Build the checkpoint fallback service from the community-maintained list by [ethPandaOps](https://github.com/ethpandaops).
    ///
    /// The list is defined in [ethPandaOps/checkpoint-fallback-service](https://github.com/ethpandaops/checkpoint-sync-health-checks/blob/master/_data/endpoints.yaml).
    pub async fn build(&mut self) -> Result<Vec<(Network, Vec<CheckpointFallbackService>)>, Error> {
        // Fetch the services
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(60))
            .tcp_keepalive(Duration::from_secs(60))
            .build()
            .map_err(|e| eyre::eyre!("Failed to create HTTP client: {}", e))?;

        let res = client
            .get(CHECKPOINT_SYNC_SERVICES_LIST)
            .send()
            .await
            .map_err(|e| eyre::eyre!("Failed to fetch checkpoint services list: {}", e))?;

        let str = res
            .text()
            .await
            .map_err(|e| eyre::eyre!("Failed to get response text: {}", e))?;

        // Parse the yaml content results.
        let list: serde_yaml::Value =
            serde_yaml::from_str(&str).map_err(|e| eyre::eyre!("Failed to parse YAML: {}", e))?;

        let mut services: Vec<(Network, Vec<CheckpointFallbackService>)> = Vec::with_capacity(3);
        let networks = ["mainnet", "holesky", "sepolia"];

        for network in networks.iter() {
            // Try to parse list of checkpoint fallback services
            let service_list = list
                .get(network)
                .ok_or_else(|| eyre::eyre!("Missing network: {}", network))?;

            let parsed: Vec<CheckpointFallbackService> =
                serde_yaml::from_value(service_list.clone())
                    .map_err(|e| eyre::eyre!("Failed to parse services for {}: {}", network, e))?;

            let network_enum = match *network {
                "mainnet" => Network::Mainnet,
                "holesky" => Network::Holesky,
                "sepolia" => Network::Sepolia,
                _ => continue,
            };

            services.push((network_enum, parsed));
        }

        self.services = services.clone();
        Ok(services)
    }

    /// Fetch the latest checkpoint from the checkpoint fallback service.
    pub async fn fetch_latest_checkpoint(&self, network: usize) -> eyre::Result<B256> {
        let services = &self.get_healthy_fallback_services(network);
        Self::fetch_latest_checkpoint_from_services(&services[..]).await
    }

    async fn query_service(endpoint: &str) -> Option<RawSlotResponse> {
        let constructed_url = Self::construct_url(endpoint);
        let res = get(&constructed_url).await.ok()?;
        let raw: RawSlotResponse = res.json().await.ok()?;
        Some(raw)
    }

    /// Fetch the latest checkpoint from a list of checkpoint fallback services.
    pub async fn fetch_latest_checkpoint_from_services(
        services: &[CheckpointFallbackService],
    ) -> eyre::Result<B256> {
        // Iterate over all mainnet checkpoint sync services and get the latest checkpoint slot for each.
        let tasks: Vec<_> = services
            .iter()
            .map(|service| async move {
                let service = service.clone();
                match Self::query_service(&service.endpoint).await {
                    Some(raw) => {
                        if raw.data.slots.is_empty() {
                            return Err(eyre::eyre!("no slots"));
                        }

                        let slot = raw
                            .data
                            .slots
                            .iter()
                            .find(|s| s.block_root.is_some())
                            .ok_or(eyre::eyre!("no valid slots"))?;

                        Ok(slot.clone())
                    }
                    None => Err(eyre::eyre!("failed to query service")),
                }
            })
            .collect();

        let slots = futures::future::join_all(tasks)
            .await
            .iter()
            .filter_map(|slot| match &slot {
                Ok(s) => Some(s.clone()),
                _ => None,
            })
            .filter(|s| s.block_root.is_some())
            .collect::<Vec<_>>();

        // Get the max epoch
        let max_epoch_slot = slots.iter().max_by_key(|x| x.epoch).ok_or(eyre::eyre!(
            "Failed to find max epoch from checkpoint slots"
        ))?;
        let max_epoch = max_epoch_slot.epoch;

        // Filter out all the slots that are not the max epoch.
        let slots = slots
            .into_iter()
            .filter(|x| x.epoch == max_epoch)
            .collect::<Vec<_>>();

        // Return the most commonly verified checkpoint.
        let checkpoints = slots
            .iter()
            .filter_map(|x| x.block_root)
            .collect::<Vec<_>>();
        let mut m: HashMap<B256, usize> = HashMap::new();
        for c in checkpoints {
            *m.entry(c).or_default() += 1;
        }
        let most_common = m.into_iter().max_by_key(|(_, v)| *v).map(|(k, _)| k);

        // Return the most commonly verified checkpoint for the latest epoch.
        most_common.ok_or_else(|| eyre::eyre!("No checkpoint found"))
    }

    /// Updates the health status of services by making test requests
    pub async fn update_health_status(&mut self) -> eyre::Result<()> {
        for (_, services) in self.services.iter_mut() {
            for service in services.iter_mut() {
                let health_status = Self::check_service_health(&service.endpoint).await;

                let now = chrono::Utc::now().to_rfc3339();
                let health_entry = Health {
                    result: health_status,
                    date: now,
                };

                // Update the service state based on health check
                service.state = health_status;

                // Add new health entry, keeping only the most recent 10
                service.health.push(health_entry);
                if service.health.len() > 10 {
                    service.health.remove(0);
                }
            }
        }

        Ok(())
    }

    /// Check if a service is healthy by making a test request
    async fn check_service_health(endpoint: &str) -> bool {
        match Self::query_service(endpoint).await {
            Some(response) => !response.data.slots.is_empty(),
            None => false,
        }
    }

    /// Returns a list of all checkpoint fallback endpoints.
    ///
    /// ### Warning
    ///
    /// These services are not healthchecked **nor** trustworthy and may act with malice by returning invalid checkpoints.
    pub fn get_all_fallback_endpoints(&self, network: usize) -> Vec<String> {
        self.services[network]
            .1
            .iter()
            .map(|service| service.endpoint.clone())
            .collect()
    }

    /// Returns a list of healthchecked checkpoint fallback services.
    ///
    /// ### Warning
    ///
    /// These services are not trustworthy and may act with malice by returning invalid checkpoints.
    pub fn get_healthy_fallback_services(&self, network: usize) -> Vec<CheckpointFallbackService> {
        self.services[network]
            .1
            .iter()
            .filter(|service| service.state)
            .cloned()
            .collect::<Vec<CheckpointFallbackService>>()
    }

    /// Lookup a network by its enum value and return its index
    pub fn get_network_index(&self, network: Network) -> Option<usize> {
        self.services.iter().position(|(net, _)| *net == network)
    }

    /// Returns the raw checkpoint fallback service objects for a given network.
    pub fn get_fallback_services(&self, network: usize) -> &Vec<CheckpointFallbackService> {
        self.services[network].1.as_ref()
    }
}

