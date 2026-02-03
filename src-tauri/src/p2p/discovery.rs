//! mDNS discovery for LAN peers
//!
//! Discovers VIBE-ON! peers on the local network using mDNS

use std::sync::Arc;

use libp2p::{mdns, Multiaddr, PeerId, Swarm};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::sync::RwLock;

use super::protocol::StreamingBehaviour;
use super::{P2PEvent, P2PState};

/// Information about a discovered peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPeer {
    /// The peer's ID (as string for serialization)
    pub peer_id: String,
    /// The peer's addresses (as strings for serialization)
    pub addresses: Vec<String>,
    /// Device name (hostname)
    pub device_name: String,
    /// Platform (desktop/mobile)
    pub platform: String,
    /// App version
    pub version: String,
    /// Whether this is a local network peer (mDNS discovered)
    pub is_local: bool,
}

impl DiscoveredPeer {
    pub fn new(peer_id: PeerId, addresses: Vec<Multiaddr>) -> Self {
        Self {
            peer_id: peer_id.to_base58(),
            addresses: addresses.iter().map(|a| a.to_string()).collect(),
            device_name: "Unknown".to_string(),
            platform: "unknown".to_string(),
            version: "unknown".to_string(),
            is_local: true,
        }
    }
    
    pub fn with_device_name(mut self, name: String) -> Self {
        self.device_name = name;
        self
    }
    
    pub fn with_platform(mut self, platform: String) -> Self {
        self.platform = platform;
        self
    }
    
    pub fn with_version(mut self, version: String) -> Self {
        self.version = version;
        self
    }
}

/// Handle mDNS events
pub async fn handle_mdns_event(
    swarm: &mut Swarm<StreamingBehaviour>,
    state: &Arc<RwLock<P2PState>>,
    event_tx: &mpsc::Sender<P2PEvent>,
    event: mdns::Event,
) {
    match event {
        mdns::Event::Discovered(peers) => {
            for (peer_id, addr) in peers {
                log::info!("mDNS discovered peer: {} at {}", peer_id, addr);
                
                // Add address to swarm
                swarm.add_peer_address(peer_id, addr.clone());
                
                // Create or update peer info
                let mut state = state.write().await;
                let peer = state.peers.entry(peer_id).or_insert_with(|| {
                    DiscoveredPeer::new(peer_id, vec![])
                });
                
                let addr_str = addr.to_string();
                if !peer.addresses.contains(&addr_str) {
                    peer.addresses.push(addr_str);
                }
                peer.is_local = true;
                
                let peer_clone = peer.clone();
                drop(state);
                
                // Notify listeners
                let _ = event_tx.send(P2PEvent::PeerDiscovered(peer_clone)).await;
            }
        }
        mdns::Event::Expired(peers) => {
            for (peer_id, addr) in peers {
                log::info!("mDNS peer expired: {} at {}", peer_id, addr);
                
                let mut state = state.write().await;
                let addr_str = addr.to_string();
                
                // Remove the specific address
                if let Some(peer) = state.peers.get_mut(&peer_id) {
                    peer.addresses.retain(|a| a != &addr_str);
                    
                    // If no addresses left, remove the peer
                    if peer.addresses.is_empty() {
                        state.peers.remove(&peer_id);
                        drop(state);
                        
                        let _ = event_tx.send(P2PEvent::PeerLost(peer_id)).await;
                    }
                }
            }
        }
    }
}
