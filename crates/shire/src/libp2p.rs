// use libp2p::{
//     futures::StreamExt,
//     gossipsub::{self, IdentTopic},
//     identity, kad, noise, ping,
//     swarm::{NetworkBehaviour, SwarmEvent},
//     tcp, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder,
// };
// use std::error::Error;
// use std::time::Duration;
// use tokio::{signal, sync::oneshot};

// /// Ethereum Foundation Go Bootnodes
// pub static MAINNET_BOOTNODES : [&str; 4] = [
//     "enode://d860a01f9722d78051619d1e2351aba3f43f943f6f00718d1b9baa4101932a1f5011f16bb2b1bb35db20d6fe28fa0bf09636d26a87d31de9ec6203eeedb1f666@18.138.108.67:30303",   // bootnode-aws-ap-southeast-1-001
//     "enode://22a8232c3abc76a16ae9d6c3b164f98775fe226f0917b0ca871128a74a8e9630b458460865bab457221f1d448dd9791d24c4e5d88786180ac185df813a68d4de@3.209.45.79:30303",     // bootnode-aws-us-east-1-001
//     "enode://2b252ab6a1d0f971d9722cb839a42cb81db019ba44c08754628ab4a823487071b5695317c8ccd085219c3a03af063495b2f1da8d18218da2d6a82981b45e6ffc@65.108.70.101:30303",   // bootnode-hetzner-hel
//     "enode://4aeb4ab6c14b23e2c4cfdce879c04b0748a20d8e9b59e25ded2a08143e265c6c25936e74cbc8e641e3312ca288673d91f2f93f8e277de3cfa444ecdaaf982052@157.90.35.166:30303",   // bootnode-hetzner-fsn
// ];

// #[derive(Debug)]
// enum LightClientEvent {
//     Ping(ping::Event),
//     Gossipsub(gossipsub::Event),
//     Kademlia(kad::Event),
// }

// impl From<ping::Event> for LightClientEvent {
//     fn from(event: ping::Event) -> Self {
//         LightClientEvent::Ping(event)
//     }
// }

// impl From<gossipsub::Event> for LightClientEvent {
//     fn from(event: gossipsub::Event) -> Self {
//         LightClientEvent::Gossipsub(event)
//     }
// }

// impl From<kad::Event> for LightClientEvent {
//     fn from(event: kad::Event) -> Self {
//         LightClientEvent::Kademlia(event)
//     }
// }

// #[derive(NetworkBehaviour)]
// #[behaviour(out_event = "LightClientEvent")]
// struct LightClientBehaviour {
//     gossipsub: gossipsub::Behaviour,
//     kademlia: kad::Behaviour<kad::store::MemoryStore>,
//     ping: ping::Behaviour,
// }

// pub struct LightClient {
//     swarm: Swarm<LightClientBehaviour>,
// }

// impl LightClientBehaviour {
//     fn new() -> Result<Self, Box<dyn Error>> {
//         let local_key = identity::Keypair::generate_ed25519();
//         let local_peer_id = PeerId::from(local_key.public());

//         // Setup Kademlia
//         let mut kad_config = kad::Config::default();
//         kad_config.set_query_timeout(Duration::from_secs(5 * 60));

//         let store = kad::store::MemoryStore::new(local_peer_id);
//         let mut kademlia = kad::Behaviour::with_config(local_peer_id, store, kad_config);

//         // Add bootnodes
//         for bootnode in MAINNET_BOOTNODES.iter() {
//             if let Ok(multiaddr) = convert_enode_to_multiaddr(bootnode) {
//                 kademlia.add_address(&local_peer_id, multiaddr);
//             }
//         }

//         // Setup gossipsub
//         let gossipsub_config = gossipsub::ConfigBuilder::default()
//             .flood_publish(true)
//             .history_length(5)
//             .validation_mode(gossipsub::ValidationMode::Anonymous)
//             .build()?;

//         let mut gossipsub =
//             gossipsub::Behaviour::new(gossipsub::MessageAuthenticity::Anonymous, gossipsub_config)?;

//         // Subscribe to light client topics
//         gossipsub.subscribe(&IdentTopic::new(
//             "/eth2/beacon_chain/light_client/finality_update/1/",
//         ))?;
//         gossipsub.subscribe(&IdentTopic::new(
//             "/eth2/beacon_chain/light_client/optimistic_update/1/",
//         ))?;

//         Ok(Self {
//             gossipsub,
//             kademlia,
//             ping: ping::Behaviour::default(),
//         })
//     }
// }

// impl LightClient {
//     pub fn new() -> Result<Self, Box<dyn Error>> {
//         let behaviour = LightClientBehaviour::new()?;

//         let mut swarm = SwarmBuilder::with_new_identity()
//             .with_tokio()
//             .with_tcp(
//                 tcp::Config::default(),
//                 noise::Config::new,
//                 yamux::Config::default,
//             )?
//             .with_behaviour(|_| Ok(behaviour))?
//             .build();

//         swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

//         Ok(Self { swarm })
//     }

//     pub async fn start(
//         &mut self,
//         mut shutdown_rx: oneshot::Receiver<()>,
//     ) -> Result<(), Box<dyn Error>> {
//         loop {
//             tokio::select! {
//                 event = self.swarm.select_next_some() => {
//                     match event {
//                         SwarmEvent::NewListenAddr { address, .. } => {
//                             println!("Listening on {:?}", address);
//                         }
//                         SwarmEvent::Behaviour(LightClientEvent::Kademlia(kad::Event::OutboundQueryProgressed {
//                             result: kad::QueryResult::GetClosestPeers(Ok(ok)),
//                             ..
//                         })) => {
//                             println!("Found closest peers: {:?}", ok.peers);
//                         }
//                         SwarmEvent::Behaviour(event) => {
//                             println!("Other event: {:?}", event);
//                         }
//                         _ => {}
//                     }
//                 }
//                 _ = &mut shutdown_rx => {
//                     println!("Received shutdown signal. Stopping...");
//                     break;
//                 }
//             }
//         }
//         Ok(())
//     }
// }

// fn convert_enode_to_multiaddr(enode: &str) -> Result<Multiaddr, Box<dyn Error>> {
//     // Parse enode URL format: enode://pubkey@ip:port
//     let parts: Vec<&str> = enode.strip_prefix("enode://").unwrap().split('@').collect();

//     let addr_port: Vec<&str> = parts[1].split(':').collect();
//     let ip = addr_port[0];
//     let port = addr_port[1];

//     // Convert to multiaddr format
//     let multiaddr = format!("/ip4/{}/tcp/{}", ip, port).parse()?;

//     Ok(multiaddr)
// }

// #[cfg(test)]
// mod tests {
//     use tokio::sync::broadcast;

//     use super::*;

//     #[tokio::test]
//     async fn test_light_client_network() -> Result<(), Box<dyn Error>> {
//         println!("Initializing Swarm...");

//         let mut client1 = LightClient::new()?;
//         let mut client2 = LightClient::new()?;

//         // Store peer IDs before moving clients
//         let client1_id = client1.swarm.local_peer_id().clone();
//         let client2_id = client2.swarm.local_peer_id().clone();

//         // Create broadcast channel for shutdown signal
//         let (shutdown_tx, _) = broadcast::channel::<()>(1);
//         let mut shutdown_rx1 = shutdown_tx.subscribe();
//         let mut shutdown_rx2 = shutdown_tx.subscribe();

//         // Create channels to receive peer discovery events
//         let (peers_tx1, mut peers_rx1) = tokio::sync::mpsc::channel(1);
//         let (peers_tx2, mut peers_rx2) = tokio::sync::mpsc::channel(1);

//         // Start both clients with peer discovery reporting
//         let client1_handle = tokio::spawn(async move {
//             loop {
//                 tokio::select! {
//                     event = client1.swarm.select_next_some() => {
//                         if let SwarmEvent::Behaviour(LightClientEvent::Kademlia(

//                             kad::Event::RoutingUpdated { peer, .. }

//                         )) = event {
//                             let _ = peers_tx1.send(peer).await;
//                         }
//                     }
//                     Ok(_) = shutdown_rx1.recv() => break,
//                     else => break,
//                 }
//             }
//         });

//         let client2_handle = tokio::spawn(async move {
//             loop {
//                 tokio::select! {
//                     event = client2.swarm.select_next_some() => {
//                         if let SwarmEvent::Behaviour(LightClientEvent::Kademlia(
//                             kad::Event::RoutingUpdated { peer, .. }
//                         )) = event {
//                             let _ = peers_tx2.send(peer).await;
//                         }
//                     }
//                     Ok(_) = shutdown_rx2.recv() => break,  // Call recv() to get a Future
//                     else => break,
//                 }
//             }
//         });
//         // Wait for peer discovery
//         let mut client1_found = false;
//         let mut client2_found = false;

//         tokio::select! {
//             _ = async {
//                 while let Some(peer) = peers_rx1.recv().await {
//                     if peer == client2_id {
//                         client2_found = true;
//                         break;
//                     }
//                 }
//                 while let Some(peer) = peers_rx2.recv().await {
//                     if peer == client1_id {
//                         client1_found = true;
//                         break;
//                     }
//                 }
//             } => {},
//             _ = tokio::time::sleep(Duration::from_secs(5)) => {},
//         }

//         // Send shutdown signals
//         let _ = shutdown_tx.send(());

//         // Wait for tasks to exit cleanly
//         let _ = client1_handle.await;
//         let _ = client2_handle.await;

//         println!("Client 1 found client 2: {}", client1_found);

//         assert!(
//             client1_found && client2_found,
//             "Peers should have discovered each other"
//         );
//         Ok(())
//     }
// }
