//! P2P Streaming Module for VIBE-ON!
//!
//! Provides lossless audio streaming over libp2p with:
//! - QUIC transport for low-latency connections
//! - mDNS discovery for LAN peers
//! - Adaptive buffering (full pre-buffer for ≤20MB, streaming for larger)
//! - Original file byte passthrough (no re-encoding)

pub mod buffer;
pub mod discovery;
pub mod protocol;
pub mod transport;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use futures::StreamExt;
use libp2p::swarm::SwarmEvent;
use libp2p::{Multiaddr, PeerId, Swarm};
use tokio::sync::{mpsc, RwLock};

use self::discovery::DiscoveredPeer;
use self::protocol::{StreamRequest, StreamingBehaviour};
use self::transport::build_swarm;

/// Commands sent to the P2P manager
#[derive(Debug, Clone)]
pub enum P2PCommand {
    /// Start streaming a track to a remote peer
    StreamToPeer {
        peer_id: PeerId,
        track_path: PathBuf,
        start_byte: u64,
    },
    /// Request a stream from a remote peer
    RequestStream {
        peer_id: PeerId,
        track_path: String,
        start_byte: u64,
    },
    /// Stop current streaming session
    StopStream,
    /// Connect to a specific peer
    ConnectPeer { multiaddr: Multiaddr },
    /// Get list of discovered peers
    GetPeers,
    /// Seek to a position (for large files, triggers re-stream)
    Seek { byte_offset: u64 },
    /// Shutdown the P2P manager
    Shutdown,
}

/// Events emitted by the P2P manager
#[derive(Debug, Clone)]
pub enum P2PEvent {
    /// A new peer was discovered
    PeerDiscovered(DiscoveredPeer),
    /// A peer was lost
    PeerLost(PeerId),
    /// Connected to a peer
    PeerConnected(PeerId),
    /// Disconnected from a peer
    PeerDisconnected(PeerId),
    /// Stream is ready (includes whether it's pre-buffered)
    StreamReady {
        peer_id: PeerId,
        format: String,
        file_size: u64,
        sample_rate: u32,
        channels: u16,
        duration_secs: f64,
        prebuffered: bool,
    },
    /// Received audio data chunk
    AudioData {
        sequence: u64,
        data: Vec<u8>,
        is_last: bool,
    },
    /// Stream ended
    StreamEnded,
    /// Error occurred
    Error(String),
    /// List of current peers
    PeerList(Vec<DiscoveredPeer>),
}

/// Information about an active stream
#[derive(Debug, Clone)]
pub struct ActiveStream {
    pub peer_id: PeerId,
    pub track_path: PathBuf,
    pub file_size: u64,
    pub bytes_sent: u64,
    pub is_sending: bool,
}

/// P2P Manager state
pub struct P2PState {
    /// Currently discovered peers
    pub peers: HashMap<PeerId, DiscoveredPeer>,
    /// Active outgoing stream (we're sending)
    pub outgoing_stream: Option<ActiveStream>,
    /// Active incoming stream (we're receiving)
    pub incoming_stream: Option<ActiveStream>,
    /// Our local peer ID
    pub local_peer_id: PeerId,
    /// Device name for discovery
    pub device_name: String,
}

impl P2PState {
    pub fn new(local_peer_id: PeerId, device_name: String) -> Self {
        Self {
            peers: HashMap::new(),
            outgoing_stream: None,
            incoming_stream: None,
            local_peer_id,
            device_name,
        }
    }
}

/// P2P Manager handles all peer-to-peer operations
pub struct P2PManager {
    /// Shared state
    state: Arc<RwLock<P2PState>>,
    /// Command sender
    cmd_tx: mpsc::Sender<P2PCommand>,
    /// Event receiver
    event_rx: mpsc::Receiver<P2PEvent>,
}

impl P2PManager {
    /// Create and start a new P2P manager
    pub async fn new(device_name: String) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (swarm, local_peer_id) = build_swarm(&device_name).await?;
        
        let state = Arc::new(RwLock::new(P2PState::new(local_peer_id, device_name)));
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let (event_tx, event_rx) = mpsc::channel(64);
        
        // Spawn the event loop
        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            run_event_loop(swarm, state_clone, cmd_rx, event_tx).await;
        });
        
        Ok(Self {
            state,
            cmd_tx,
            event_rx,
        })
    }
    
    /// Get the local peer ID
    pub async fn local_peer_id(&self) -> PeerId {
        self.state.read().await.local_peer_id
    }
    
    /// Get list of discovered peers
    pub async fn get_peers(&self) -> Vec<DiscoveredPeer> {
        self.state.read().await.peers.values().cloned().collect()
    }
    
    /// Send a command to the P2P manager
    pub async fn send_command(&self, cmd: P2PCommand) -> Result<(), mpsc::error::SendError<P2PCommand>> {
        self.cmd_tx.send(cmd).await
    }
    
    /// Try to receive the next event (non-blocking)
    pub fn try_recv_event(&mut self) -> Option<P2PEvent> {
        self.event_rx.try_recv().ok()
    }
    
    /// Receive the next event (blocking)
    pub async fn recv_event(&mut self) -> Option<P2PEvent> {
        self.event_rx.recv().await
    }
    
    /// Request a stream from a peer
    pub async fn request_stream(
        &self,
        peer_id: PeerId,
        track_path: String,
        start_byte: u64,
    ) -> Result<(), mpsc::error::SendError<P2PCommand>> {
        self.send_command(P2PCommand::RequestStream {
            peer_id,
            track_path,
            start_byte,
        }).await
    }
    
    /// Stream a track to a peer
    pub async fn stream_to_peer(
        &self,
        peer_id: PeerId,
        track_path: PathBuf,
        start_byte: u64,
    ) -> Result<(), mpsc::error::SendError<P2PCommand>> {
        self.send_command(P2PCommand::StreamToPeer {
            peer_id,
            track_path,
            start_byte,
        }).await
    }
    
    /// Stop current stream
    pub async fn stop_stream(&self) -> Result<(), mpsc::error::SendError<P2PCommand>> {
        self.send_command(P2PCommand::StopStream).await
    }
    
    /// Connect to a peer by multiaddr
    pub async fn connect_peer(&self, multiaddr: Multiaddr) -> Result<(), mpsc::error::SendError<P2PCommand>> {
        self.send_command(P2PCommand::ConnectPeer { multiaddr }).await
    }
}

/// Main event loop for the P2P swarm
async fn run_event_loop(
    mut swarm: Swarm<StreamingBehaviour>,
    state: Arc<RwLock<P2PState>>,
    mut cmd_rx: mpsc::Receiver<P2PCommand>,
    event_tx: mpsc::Sender<P2PEvent>,
) {
    // Start listening on all interfaces
    let listen_addr: Multiaddr = "/ip4/0.0.0.0/udp/0/quic-v1".parse().unwrap();
    if let Err(e) = swarm.listen_on(listen_addr) {
        let _ = event_tx.send(P2PEvent::Error(format!("Failed to listen: {}", e))).await;
        return;
    }
    
    loop {
        tokio::select! {
            // Handle incoming swarm events
            event = swarm.select_next_some() => {
                handle_swarm_event(&mut swarm, &state, &event_tx, event).await;
            }
            
            // Handle commands
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    P2PCommand::Shutdown => {
                        break;
                    }
                    P2PCommand::ConnectPeer { multiaddr } => {
                        if let Err(e) = swarm.dial(multiaddr.clone()) {
                            let _ = event_tx.send(P2PEvent::Error(format!("Failed to dial {}: {}", multiaddr, e))).await;
                        }
                    }
                    P2PCommand::GetPeers => {
                        let peers: Vec<_> = state.read().await.peers.values().cloned().collect();
                        let _ = event_tx.send(P2PEvent::PeerList(peers)).await;
                    }
                    P2PCommand::RequestStream { peer_id, track_path, start_byte } => {
                        // Send stream request to peer
                        let request = StreamRequest::RequestTrack {
                            track_path,
                            start_byte,
                        };
                        swarm.behaviour_mut().streaming.send_request(&peer_id, request);
                    }
                    P2PCommand::StreamToPeer { peer_id, track_path, start_byte: _ } => {
                        // Start streaming to peer (handled in protocol)
                        let mut state = state.write().await;
                        state.outgoing_stream = Some(ActiveStream {
                            peer_id,
                            track_path: track_path.clone(),
                            file_size: 0,
                            bytes_sent: 0,
                            is_sending: true,
                        });
                    }
                    P2PCommand::StopStream => {
                        let mut state = state.write().await;
                        state.outgoing_stream = None;
                        state.incoming_stream = None;
                        let _ = event_tx.send(P2PEvent::StreamEnded).await;
                    }
                    P2PCommand::Seek { byte_offset } => {
                        // For large files, we need to request a new stream from the offset
                        let state = state.read().await;
                        if let Some(ref stream) = state.incoming_stream {
                            let peer_id = stream.peer_id;
                            let _track_path = stream.track_path.to_string_lossy().to_string();
                            drop(state);
                            
                            let request = StreamRequest::Seek { byte_offset };
                            swarm.behaviour_mut().streaming.send_request(&peer_id, request);
                        }
                    }
                }
            }
        }
    }
}

/// Handle swarm events
async fn handle_swarm_event(
    swarm: &mut Swarm<StreamingBehaviour>,
    state: &Arc<RwLock<P2PState>>,
    event_tx: &mpsc::Sender<P2PEvent>,
    event: SwarmEvent<protocol::StreamingBehaviourEvent>,
) {
    match event {
        SwarmEvent::NewListenAddr { address, .. } => {
            log::info!("Listening on {}", address);
        }
        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
            let _ = event_tx.send(P2PEvent::PeerConnected(peer_id)).await;
        }
        SwarmEvent::ConnectionClosed { peer_id, .. } => {
            let _ = event_tx.send(P2PEvent::PeerDisconnected(peer_id)).await;
        }
        SwarmEvent::Behaviour(behaviour_event) => {
            match behaviour_event {
                protocol::StreamingBehaviourEvent::Mdns(mdns_event) => {
                    discovery::handle_mdns_event(swarm, state, event_tx, mdns_event).await;
                }
                protocol::StreamingBehaviourEvent::Streaming(streaming_event) => {
                    protocol::handle_streaming_event(swarm, state, event_tx, streaming_event).await;
                }
                protocol::StreamingBehaviourEvent::Identify(identify_event) => {
                    // Handle identify events for peer info
                    if let libp2p::identify::Event::Received { peer_id, info, connection_id: _ } = identify_event {
                        log::debug!("Identified peer {}: {:?}", peer_id, info.agent_version);
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// Utility function to get hostname
pub fn get_device_name() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "VIBE-ON Desktop".to_string())
}
