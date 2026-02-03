//! Transport layer for P2P connections
//!
//! Builds the libp2p swarm with:
//! - QUIC transport (low-latency, reliable)
//! - Noise encryption
//! - Yamux multiplexing

use std::time::Duration;

use libp2p::{
    autonat, dcutr, identify, mdns, noise,
    yamux, PeerId, Swarm, SwarmBuilder,
};

use super::protocol::{StreamingBehaviour, new_streaming_protocol};

/// Build the libp2p swarm with all required protocols
pub async fn build_swarm(
    _device_name: &str,
) -> Result<(Swarm<StreamingBehaviour>, PeerId), Box<dyn std::error::Error + Send + Sync>> {
    let swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_quic()
        .with_relay_client(noise::Config::new, yamux::Config::default)?
        .with_behaviour(|keypair, relay_behaviour| {
            let local_peer_id = keypair.public().to_peer_id();
            
            // mDNS for local network discovery
            let mdns = mdns::tokio::Behaviour::new(
                mdns::Config::default(),
                local_peer_id,
            )?;
            
            // Identify protocol for peer info exchange
            let identify = identify::Behaviour::new(
                identify::Config::new(
                    "/vibe-on/1.0.0".to_string(),
                    keypair.public(),
                )
                .with_agent_version(format!("vibe-on/{}", env!("CARGO_PKG_VERSION")))
                .with_push_listen_addr_updates(true),
            );
            
            // AutoNAT for public address detection
            let autonat = autonat::Behaviour::new(
                local_peer_id,
                autonat::Config::default(),
            );
            
            // DCUtR for hole punching
            let dcutr = dcutr::Behaviour::new(local_peer_id);
            
            // Streaming protocol
            let streaming = new_streaming_protocol();
            
            Ok(StreamingBehaviour {
                mdns,
                identify,
                autonat,
                dcutr,
                relay: relay_behaviour,
                streaming,
            })
        })?
        .with_swarm_config(|cfg| {
            // Increased from 60s to 300s to prevent audio streaming disconnects during pauses
            cfg.with_idle_connection_timeout(Duration::from_secs(300))
        })
        .build();
    
    let local_peer_id = *swarm.local_peer_id();
    
    Ok((swarm, local_peer_id))
}
