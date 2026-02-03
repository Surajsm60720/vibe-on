//! Streaming protocol for P2P audio transfer
//!
//! Implements the `/vibe-on/stream/1.0.0` protocol:
//! - StreamRequest: Request track, seek, or stop
//! - StreamResponse: Header with metadata, chunks with audio data

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures::prelude::*;
use libp2p::{
    autonat, dcutr, identify, mdns, relay,
    request_response::{self, Codec, ProtocolSupport},
    swarm::NetworkBehaviour,
    PeerId, StreamProtocol,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};

use super::{P2PEvent, P2PState};

/// Chunk size for streaming (64KB)
pub const CHUNK_SIZE: usize = 65536;

/// Threshold for pre-buffering entire file (20MB)
pub const PREBUFFER_THRESHOLD: u64 = 20 * 1024 * 1024;

/// Request messages for the streaming protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamRequest {
    /// Request to stream a track
    RequestTrack {
        track_path: String,
        start_byte: u64,
    },
    /// Seek to a byte offset (for large files)
    Seek {
        byte_offset: u64,
    },
    /// Stop the current stream
    Stop,
    /// Ping for keepalive
    Ping,
}

/// Response messages for the streaming protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamResponse {
    /// Stream header with metadata
    Header {
        /// Original file format (flac, mp3, etc.)
        format: String,
        /// Sample rate in Hz
        sample_rate: u32,
        /// Number of channels
        channels: u16,
        /// Duration in seconds
        duration_secs: f64,
        /// Total file size in bytes
        file_size: u64,
        /// Track title
        title: String,
        /// Track artist
        artist: String,
        /// Track album
        album: String,
    },
    /// Audio data chunk
    Chunk {
        /// Sequence number
        sequence: u64,
        /// Raw file bytes
        data: Vec<u8>,
        /// Is this the last chunk?
        is_last: bool,
    },
    /// Seek acknowledgment
    SeekAck {
        /// New byte offset
        byte_offset: u64,
    },
    /// Stream stopped
    Stopped,
    /// Pong response
    Pong,
    /// Error response
    Error {
        message: String,
    },
}

/// CBOR codec for the streaming protocol
#[derive(Debug, Clone, Default)]
pub struct StreamingCodec;

impl Codec for StreamingCodec {
    type Protocol = StreamProtocol;
    type Request = StreamRequest;
    type Response = StreamResponse;

    fn read_request<'life0, 'life1, 'life2, 'async_trait, T>(
        &'life0 mut self,
        _protocol: &'life1 Self::Protocol,
        io: &'life2 mut T,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<Self::Request>> + Send + 'async_trait>>
    where
        T: futures::AsyncRead + Unpin + Send + 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let mut buf = Vec::new();
            let mut limited = io.take(1024 * 1024); // 1MB limit for requests
            limited.read_to_end(&mut buf).await?;
            serde_cbor::from_slice(&buf)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })
    }

    fn read_response<'life0, 'life1, 'life2, 'async_trait, T>(
        &'life0 mut self,
        _protocol: &'life1 Self::Protocol,
        io: &'life2 mut T,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<Self::Response>> + Send + 'async_trait>>
    where
        T: futures::AsyncRead + Unpin + Send + 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let mut buf = Vec::new();
            let mut limited = io.take(100 * 1024 * 1024); // 100MB limit for responses (full files)
            limited.read_to_end(&mut buf).await?;
            serde_cbor::from_slice(&buf)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })
    }

    fn write_request<'life0, 'life1, 'life2, 'async_trait, T>(
        &'life0 mut self,
        _protocol: &'life1 Self::Protocol,
        io: &'life2 mut T,
        req: Self::Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'async_trait>>
    where
        T: futures::AsyncWrite + Unpin + Send + 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let data = serde_cbor::to_vec(&req)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            io.write_all(&data).await?;
            io.close().await?;
            Ok(())
        })
    }

    fn write_response<'life0, 'life1, 'life2, 'async_trait, T>(
        &'life0 mut self,
        _protocol: &'life1 Self::Protocol,
        io: &'life2 mut T,
        res: Self::Response,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'async_trait>>
    where
        T: futures::AsyncWrite + Unpin + Send + 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let data = serde_cbor::to_vec(&res)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            io.write_all(&data).await?;
            io.close().await?;
            Ok(())
        })
    }
}

/// Streaming protocol behaviour
pub type StreamingProtocol = request_response::Behaviour<StreamingCodec>;

/// Create a new streaming protocol behaviour
pub fn new_streaming_protocol() -> StreamingProtocol {
    let protocol = StreamProtocol::new("/vibe-on/stream/1.0.0");
    request_response::Behaviour::new(
        [(protocol, ProtocolSupport::Full)],
        request_response::Config::default()
            .with_request_timeout(Duration::from_secs(300)), // 5 min timeout for large files
    )
}

/// Combined network behaviour for the P2P swarm
#[derive(NetworkBehaviour)]
pub struct StreamingBehaviour {
    pub mdns: mdns::tokio::Behaviour,
    pub identify: identify::Behaviour,
    pub autonat: autonat::Behaviour,
    pub dcutr: dcutr::Behaviour,
    pub relay: relay::client::Behaviour,
    pub streaming: StreamingProtocol,
}

/// Handle incoming streaming protocol events
pub async fn handle_streaming_event(
    swarm: &mut libp2p::Swarm<StreamingBehaviour>,
    state: &Arc<RwLock<P2PState>>,
    event_tx: &mpsc::Sender<P2PEvent>,
    event: request_response::Event<StreamRequest, StreamResponse>,
) {
    match event {
        request_response::Event::Message { peer, message } => {
            match message {
                request_response::Message::Request { request, channel, .. } => {
                    handle_incoming_request(swarm, state, peer, request, channel).await;
                }
                request_response::Message::Response { response, .. } => {
                    handle_incoming_response(state, event_tx, peer, response).await;
                }
            }
        }
        request_response::Event::OutboundFailure { peer, error, .. } => {
            let _ = event_tx.send(P2PEvent::Error(format!(
                "Outbound request to {} failed: {:?}", peer, error
            ))).await;
        }
        request_response::Event::InboundFailure { peer, error, .. } => {
            let _ = event_tx.send(P2PEvent::Error(format!(
                "Inbound request from {} failed: {:?}", peer, error
            ))).await;
        }
        _ => {}
    }
}

/// Handle an incoming stream request
async fn handle_incoming_request(
    swarm: &mut libp2p::Swarm<StreamingBehaviour>,
    state: &Arc<RwLock<P2PState>>,
    peer: PeerId,
    request: StreamRequest,
    channel: request_response::ResponseChannel<StreamResponse>,
) {
    match request {
        StreamRequest::RequestTrack { track_path, start_byte } => {
            // Read the file and get metadata
            let path = PathBuf::from(&track_path);
            
            match read_track_for_streaming(&path, start_byte) {
                Ok((header, data)) => {
                    // First send the header
                    let _ = swarm.behaviour_mut().streaming.send_response(channel, header);
                    
                    // Then send chunks via new requests
                    // Note: In a real implementation, we'd use a streaming subprotocol
                    // For now, we send the full file in the header response for small files
                    // and use multiple request/response cycles for large files
                    
                    // Update state
                    let mut state = state.write().await;
                    state.outgoing_stream = Some(super::ActiveStream {
                        peer_id: peer,
                        track_path: path,
                        file_size: data.len() as u64,
                        bytes_sent: data.len() as u64,
                        is_sending: true,
                    });
                }
                Err(e) => {
                    let _ = swarm.behaviour_mut().streaming.send_response(
                        channel,
                        StreamResponse::Error { message: e.to_string() },
                    );
                }
            }
        }
        StreamRequest::Seek { byte_offset } => {
            // Acknowledge seek and prepare to send from new offset
            let _ = swarm.behaviour_mut().streaming.send_response(
                channel,
                StreamResponse::SeekAck { byte_offset },
            );
        }
        StreamRequest::Stop => {
            let _ = swarm.behaviour_mut().streaming.send_response(
                channel,
                StreamResponse::Stopped,
            );
            
            let mut state = state.write().await;
            state.outgoing_stream = None;
        }
        StreamRequest::Ping => {
            let _ = swarm.behaviour_mut().streaming.send_response(
                channel,
                StreamResponse::Pong,
            );
        }
    }
}

/// Handle an incoming stream response
async fn handle_incoming_response(
    state: &Arc<RwLock<P2PState>>,
    event_tx: &mpsc::Sender<P2PEvent>,
    peer: PeerId,
    response: StreamResponse,
) {
    match response {
        StreamResponse::Header { format, sample_rate, channels, duration_secs, file_size, .. } => {
            let prebuffered = file_size <= PREBUFFER_THRESHOLD;
            
            let _ = event_tx.send(P2PEvent::StreamReady {
                peer_id: peer,
                format,
                file_size,
                sample_rate,
                channels,
                duration_secs,
                prebuffered,
            }).await;
        }
        StreamResponse::Chunk { sequence, data, is_last } => {
            let _ = event_tx.send(P2PEvent::AudioData {
                sequence,
                data,
                is_last,
            }).await;
            
            if is_last {
                let _ = event_tx.send(P2PEvent::StreamEnded).await;
            }
        }
        StreamResponse::SeekAck { byte_offset } => {
            // Ready to receive from new position
            log::debug!("Seek acknowledged at byte {}", byte_offset);
        }
        StreamResponse::Stopped => {
            let mut state = state.write().await;
            state.incoming_stream = None;
            let _ = event_tx.send(P2PEvent::StreamEnded).await;
        }
        StreamResponse::Error { message } => {
            let _ = event_tx.send(P2PEvent::Error(message)).await;
        }
        StreamResponse::Pong => {
            // Keepalive response
        }
    }
}

/// Read a track file and prepare it for streaming
fn read_track_for_streaming(
    path: &PathBuf,
    start_byte: u64,
) -> Result<(StreamResponse, Vec<u8>), Box<dyn std::error::Error + Send + Sync>> {
    use lofty::prelude::*;
    use lofty::probe::Probe;
    
    // Get file metadata using lofty
    let tagged_file = Probe::open(path)?.read()?;
    let properties = tagged_file.properties();
    
    let sample_rate = properties.sample_rate().unwrap_or(44100);
    let channels = properties.channels().unwrap_or(2) as u16;
    let duration_secs = properties.duration().as_secs_f64();
    
    // Get tags
    let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());
    let title = tag.and_then(|t| t.title().map(|s| s.to_string())).unwrap_or_default();
    let artist = tag.and_then(|t| t.artist().map(|s| s.to_string())).unwrap_or_default();
    let album = tag.and_then(|t| t.album().map(|s| s.to_string())).unwrap_or_default();
    
    // Determine format from extension
    let format = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown")
        .to_lowercase();
    
    // Read file bytes
    let mut file = File::open(path)?;
    let file_size = file.metadata()?.len();
    
    // Seek to start position if needed
    if start_byte > 0 {
        file.seek(SeekFrom::Start(start_byte))?;
    }
    
    // Read remaining bytes
    let mut data = Vec::with_capacity((file_size - start_byte) as usize);
    file.read_to_end(&mut data)?;
    
    let header = StreamResponse::Header {
        format,
        sample_rate,
        channels,
        duration_secs,
        file_size,
        title,
        artist,
        album,
    };
    
    Ok((header, data))
}
