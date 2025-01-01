use libp2p::{
    core::transport::MemoryTransport, gossipsub::{
        self, MessageAuthenticity, ValidationMode 
    }, identity::Keypair, swarm::behaviour, Multiaddr, PeerId
};

mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};
     
    #[test]
    fn check() {
        let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| panic!("unreachable"))
        .as_secs();

        println!("Timestamp: {}", timestamp);
    }
}