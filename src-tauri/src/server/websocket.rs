//! WebSocket handler for real-time control

use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use super::{ConnectedClient, ServerEvent, ServerState};

/// Client to server messages
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientMessage {
    /// Initial handshake
    Hello { client_name: String },
    /// Request current status
    GetStatus,
    /// Playback controls
    Play,
    Pause,
    Resume,
    Stop,
    Next,
    Previous,
    /// Seek to position
    Seek { position_secs: f64 },
    /// Set volume (0.0 - 1.0)
    SetVolume { volume: f64 },
    /// Toggle shuffle
    ToggleShuffle,
    /// Cycle repeat mode
    CycleRepeat,
    /// Play a specific track
    PlayTrack { path: String },
    /// Play an album
    PlayAlbum { album: String, artist: String },
    /// Play an artist
    PlayArtist { artist: String },
    /// Add track to queue
    AddToQueue { path: String },
    /// Set entire queue
    SetQueue { paths: Vec<String> },
    /// Toggle favorite
    ToggleFavorite { path: String },
    /// Get lyrics
    GetLyrics,
    /// Request audio streaming to mobile
    RequestStreamToMobile,
    /// Stop streaming to mobile
    StopStreamToMobile,
    /// Mobile is ready to receive stream
    HandoffReady,
    /// Network stats from mobile
    NetworkStats { buffer_ms: u32, throughput_kbps: u32 },
    /// WebRTC signaling: offer
    WebrtcOffer { target_peer_id: String, sdp: String },
    /// WebRTC signaling: answer
    WebrtcAnswer { target_peer_id: String, sdp: String },
    /// WebRTC signaling: ICE candidate
    IceCandidate { target_peer_id: String, candidate: String },
    /// Ping for keepalive
    Ping,
}

/// Server to client messages
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ServerMessage {
    /// Welcome message after hello
    Welcome {
        client_id: String,
        server_name: String,
        version: String,
    },
    /// Connection established (what mobile expects)
    #[serde(rename = "Connected")]
    Connected {
        #[serde(rename = "clientId")]
        client_id: String,
    },
    /// Current media session state
    #[serde(rename_all = "camelCase")]
    MediaSession {
        track_id: String,
        title: String,
        artist: String,
        album: String,
        duration: f64,
        cover_url: Option<String>,
        is_playing: bool,
        position: f64,
        timestamp: u64,
    },
    /// Player status
    #[serde(rename_all = "camelCase")]
    Status {
        volume: f64,
        shuffle: bool,
        repeat_mode: String,
        output: String,
    },
    /// Position update (mapped to PlaybackState)
    #[serde(rename = "PlaybackState")]
    #[serde(rename_all = "camelCase")]
    PlaybackState {
        #[serde(rename = "is_playing")]
        is_playing: bool,
        position: f64,
        volume: f64,
    },
    /// Lyrics data
    #[serde(rename_all = "camelCase")]
    Lyrics {
        track_path: String,
        has_synced: bool,
        synced_lyrics: Option<String>,
        plain_lyrics: Option<String>,
        instrumental: bool,
    },
    /// Prepare for audio handoff
    #[serde(rename_all = "camelCase")]
    HandoffPrepare {
        sample: u64,
        url: String,
    },
    /// Commit handoff (start playing)
    HandoffCommit,
    /// Stream stopped
    StreamStopped,
    /// Queue update
    #[serde(rename = "QueueUpdate")]
    #[serde(rename_all = "camelCase")]
    QueueUpdate {
        queue: Vec<String>,
        #[serde(rename = "current_index")]
        current_index: i32,
    },
    /// WebRTC signaling relay
    #[serde(rename = "WebRTCOffer")]
    #[serde(rename_all = "camelCase")]
    WebRTCOffer {
        #[serde(rename = "from_client_id")]
        from_peer_id: String,
        #[serde(rename = "offer")]
        sdp: String,
    },
    /// WebRTC answer relay
    #[serde(rename = "WebRTCAnswer")]
    #[serde(rename_all = "camelCase")]
    WebRTCAnswer {
        #[serde(rename = "to_client_id")]
        to_peer_id: String,
        #[serde(rename = "answer")]
        sdp: String,
    },
    /// ICE candidate relay
    #[serde(rename = "ICECandidate")]
    #[serde(rename_all = "camelCase")]
    ICECandidate {
        #[serde(rename = "from_client_id")]
        from_peer_id: String,
        candidate: String,
    },
    /// Error message
    #[serde(rename = "Error")]
    Error { message: String },
    /// Pong response
    Pong,
}

impl From<ServerEvent> for ServerMessage {
    fn from(event: ServerEvent) -> Self {
        match event {
            ServerEvent::MediaSession {
                track_id, title, artist, album, duration, cover_url, is_playing, position, timestamp
            } => ServerMessage::MediaSession {
                track_id, title, artist, album, duration, cover_url, is_playing, position, timestamp
            },
            ServerEvent::PositionUpdate { position, timestamp: _ } => {
                // Map to PlaybackState for mobile
                // Note: We don't have is_playing/volume here, so we send defaults/nulls
                // ideally PositionUpdate should carry more info, or we assume mobile merges state
                ServerMessage::PlaybackState {
                    is_playing: true, // If we are sending position updates, we are likely playing
                    position,
                    volume: 1.0, // Backend volume is usually handled in Status event
                }
            }
            ServerEvent::Status { volume, shuffle, repeat_mode, output } => {
                ServerMessage::Status { volume, shuffle, repeat_mode, output }
            }
            ServerEvent::QueueUpdate { tracks } => {
                ServerMessage::QueueUpdate { queue: tracks.iter().map(|t| t.path.clone()).collect(), current_index: 0 } // Mobile expects list of strings (paths)
            }
            ServerEvent::Lyrics { track_path, has_synced, synced_lyrics, plain_lyrics, instrumental } => {
                ServerMessage::Lyrics { track_path, has_synced, synced_lyrics, plain_lyrics, instrumental }
            }
            ServerEvent::HandoffPrepare { sample, url } => {
                ServerMessage::HandoffPrepare { sample, url }
            }
            ServerEvent::HandoffCommit => ServerMessage::HandoffCommit,
            ServerEvent::StreamStopped => ServerMessage::StreamStopped,
            ServerEvent::WebrtcOffer { from_peer_id, sdp } => {
                ServerMessage::WebRTCOffer { from_peer_id, sdp }
            }
            ServerEvent::WebrtcAnswer { target_peer_id, sdp } => {
                ServerMessage::WebRTCAnswer { to_peer_id: target_peer_id, sdp }
            }
            ServerEvent::IceCandidate { from_peer_id, candidate } => {
                ServerMessage::ICECandidate { from_peer_id, candidate }
            }
            ServerEvent::Error { message } => ServerMessage::Error { message },
            ServerEvent::Pong => ServerMessage::Pong,
        }
    }
}

/// WebSocket upgrade handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ServerState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle a WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<ServerState>) {
    let (mut sender, mut receiver) = socket.split();
    
    // Subscribe to broadcast events
    let mut event_rx = state.event_tx.subscribe();
    
    // Generate client ID
    let client_id = uuid::Uuid::new_v4().to_string();
    let _client_id_clone = client_id.clone();
    let client_id_for_cleanup = client_id.clone();
    let app_handle = state.app_handle.clone();
    
    // Create channel for keepalive pings
    let (ping_tx, mut ping_rx) = tokio::sync::mpsc::channel::<()>(1);
    
    // Create channel for direct replies from handle_client_message
    let (reply_tx, mut reply_rx) = tokio::sync::mpsc::channel::<ServerMessage>(32);

    // Spawn task to forward events to client and handle keepalive
    let send_task = tokio::spawn(async move {
        let mut keepalive_interval = tokio::time::interval(std::time::Duration::from_secs(30));
        
        loop {
            tokio::select! {
                // Forward broadcast events
                event = event_rx.recv() => {
                    match event {
                        Ok(event) => {
                            let msg: ServerMessage = event.into();
                            let json = serde_json::to_string(&msg).unwrap();
                            if sender.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                // Forward direct replies
                reply = reply_rx.recv() => {
                    match reply {
                        Some(msg) => {
                             let json = serde_json::to_string(&msg).unwrap();
                             if sender.send(Message::Text(json.into())).await.is_err() {
                                 break;
                             }
                        }
                        None => break,
                    }
                }
                // Send keepalive ping every 30 seconds
                _ = keepalive_interval.tick() => {
                    let ping_msg = serde_json::to_string(&ServerMessage::Pong).unwrap();
                    if sender.send(Message::Text(ping_msg.into())).await.is_err() {
                        break;
                    }
                }
                // Handle close signal
                _ = ping_rx.recv() => {
                    break;
                }
            }
        }
    });
    
    // Handle incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        handle_client_message(&state, &client_id, client_msg, &reply_tx).await;
                    }
                    Err(e) => {
                        log::warn!("Invalid WebSocket message: {}", e);
                        state.broadcast(ServerEvent::Error {
                            message: format!("Invalid message format: {}", e),
                        });
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    
    // Cleanup
    let _ = ping_tx.send(()).await; // Signal send task to stop
    send_task.abort();
    
    // Remove client from list and emit disconnect event
    let mut clients = state.clients.write().await;
    let disconnected_client = clients.iter().find(|c| c.id == client_id_for_cleanup).cloned();
    clients.retain(|c| c.id != client_id_for_cleanup);
    
    // Emit disconnect event to frontend
    if let Some(client) = disconnected_client {
        let _ = app_handle.emit("mobile_client_disconnected", serde_json::json!({
            "client_id": client.id,
            "client_name": client.name,
        }));
        log::info!("Mobile client disconnected: {} ({})", client.name, client.id);
    }
}

/// Handle a client message
async fn handle_client_message(
    state: &Arc<ServerState>,
    client_id: &str,
    msg: ClientMessage,
    reply_tx: &tokio::sync::mpsc::Sender<ServerMessage>,
) {
    let app_state = state.app_state();
    
    match msg {
        ClientMessage::Hello { client_name } => {
            // Add client to list
            let client = ConnectedClient {
                id: client_id.to_string(),
                name: client_name.clone(),
                connected_at: std::time::Instant::now(),
            };
            state.clients.write().await.push(client);
            
            // Emit connection event to frontend
            let _ = state.app_handle.emit("mobile_client_connected", serde_json::json!({
                "client_id": client_id,
                "client_name": client_name,
            }));
            log::info!("Mobile client connected: {} ({})", client_name, client_id);
            
            // Send Connected event to confirm handshake
            let _ = reply_tx.send(ServerMessage::Connected { 
                client_id: client_id.to_string() 
            }).await;

            // Send current status
            send_current_status_internal(state, &app_state).await;
        }
        
        ClientMessage::GetStatus => {
            send_current_status_internal(state, &app_state).await;
        }
        
        ClientMessage::Play => {
            if let Ok(mut player_guard) = app_state.player.lock() {
                if let Some(ref mut player) = *player_guard {
                    player.resume();
                }
            }
            send_current_status_internal(state, &app_state).await;
        }
        
        ClientMessage::Pause | ClientMessage::Stop => {
            if let Ok(mut player_guard) = app_state.player.lock() {
                if let Some(ref mut player) = *player_guard {
                    player.pause();
                }
            }
            send_current_status_internal(state, &app_state).await;
        }
        
        ClientMessage::Resume => {
            if let Ok(mut player_guard) = app_state.player.lock() {
                if let Some(ref mut player) = *player_guard {
                    player.resume();
                }
            }
            send_current_status_internal(state, &app_state).await;
        }
        
        ClientMessage::Next => {
            // TODO: Implement queue navigation
            state.broadcast(ServerEvent::Error {
                message: "Next track not implemented yet".to_string(),
            });
        }
        
        ClientMessage::Previous => {
            // TODO: Implement queue navigation
            state.broadcast(ServerEvent::Error {
                message: "Previous track not implemented yet".to_string(),
            });
        }
        
        ClientMessage::Seek { position_secs } => {
            if let Ok(mut player_guard) = app_state.player.lock() {
                if let Some(ref mut player) = *player_guard {
                    player.seek(position_secs);
                }
            }
            send_current_status_internal(state, &app_state).await;
        }
        
        ClientMessage::SetVolume { volume } => {
            if let Ok(mut player_guard) = app_state.player.lock() {
                if let Some(ref mut player) = *player_guard {
                    player.set_volume(volume as f32);
                }
            }
            send_current_status_internal(state, &app_state).await;
        }
        
        ClientMessage::PlayTrack { path } => {
            if let Ok(mut player_guard) = app_state.player.lock() {
                if let Some(ref mut player) = *player_guard {
                    let _ = player.play_file(&path);
                }
            }
            send_current_status_internal(state, &app_state).await;
        }
        
        ClientMessage::RequestStreamToMobile => {
            // Get current position and prepare handoff
            let (sample, url) = {
                let player_guard = app_state.player.lock().ok();
                let position = player_guard
                    .as_ref()
                    .and_then(|p| p.as_ref())
                    .map(|p| p.get_status().position_secs)
                    .unwrap_or(0.0);
                
                // Calculate sample position (assuming 44.1kHz)
                let sample = (position * 44100.0) as u64;
                let url = format!("http://{}:{}/stream?start={}", 
                    local_ip().unwrap_or("127.0.0.1".to_string()),
                    state.config.port,
                    sample
                );
                (sample, url)
            };
            
            state.broadcast(ServerEvent::HandoffPrepare { sample, url });
        }
        
        ClientMessage::HandoffReady => {
            // Pause desktop playback and commit handoff
            if let Ok(mut player_guard) = app_state.player.lock() {
                if let Some(ref mut player) = *player_guard {
                    player.pause();
                }
            }
            state.broadcast(ServerEvent::HandoffCommit);
        }
        
        ClientMessage::StopStreamToMobile => {
            // Resume desktop playback
            if let Ok(mut player_guard) = app_state.player.lock() {
                if let Some(ref mut player) = *player_guard {
                    player.resume();
                }
            }
            state.broadcast(ServerEvent::StreamStopped);
        }
        
        ClientMessage::Ping => {
            let _ = reply_tx.send(ServerMessage::Pong).await;
        }

        ClientMessage::WebrtcOffer { target_peer_id: _, sdp } => {
            // Broadcast offer to all (filtering usually happens on client or server should unicast)
            // For now, broadcasting with from_id
            state.broadcast(ServerEvent::WebrtcOffer { 
                from_peer_id: client_id.to_string(), 
                sdp 
            });
        }
        
        ClientMessage::WebrtcAnswer { target_peer_id, sdp } => {
             state.broadcast(ServerEvent::WebrtcAnswer { 
                target_peer_id, 
                sdp 
            });
        }
        
        ClientMessage::IceCandidate { target_peer_id: _, candidate } => {
            state.broadcast(ServerEvent::IceCandidate { 
                from_peer_id: client_id.to_string(), 
                candidate 
            });
        }
        
        // TODO: Implement remaining messages
        _ => {
            log::debug!("Unhandled message type");
        }
    }
}

/// Send current playback status to all clients (internal use)
async fn send_current_status_internal(state: &Arc<ServerState>, app_state: &tauri::State<'_, crate::AppState>) {
    let (track_id, title, artist, album, duration, cover_url, is_playing, position, volume) = {
        if let Ok(player_guard) = app_state.player.lock() {
            if let Some(ref player) = *player_guard {
                let status = player.get_status();
                let is_playing = status.state == crate::audio::PlayerState::Playing;
                let position = status.position_secs;
                let volume = status.volume;
                
                if let Some(ref track) = status.track {
                    (
                        track.path.clone(),
                        track.title.clone(),
                        track.artist.clone(),
                        track.album.clone(),
                        track.duration_secs,
                        Some(format!("/cover/{}", urlencoding::encode(&track.path))),
                        is_playing,
                        position,
                        volume,
                    )
                } else {
                    ("".to_string(), "".to_string(), "".to_string(), "".to_string(), 0.0, None, false, 0.0, volume)
                }
            } else {
                ("".to_string(), "".to_string(), "".to_string(), "".to_string(), 0.0, None, false, 0.0, 1.0)
            }
        } else {
            ("".to_string(), "".to_string(), "".to_string(), "".to_string(), 0.0, None, false, 0.0, 1.0)
        }
    };
    
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    state.broadcast(ServerEvent::MediaSession {
        track_id,
        title,
        artist,
        album,
        duration,
        cover_url,
        is_playing,
        position,
        timestamp,
    });
    
    state.broadcast(ServerEvent::Status {
        volume: volume as f64,
        shuffle: false,
        repeat_mode: "off".to_string(),
        output: "desktop".to_string(),
    });
}

/// Send current playback status (public, for periodic broadcasting from mod.rs)
pub async fn send_current_status_with_handle(state: &Arc<ServerState>, app_handle: &AppHandle) {
    let app_state: tauri::State<'_, crate::AppState> = app_handle.state();
    send_current_status_internal(state, &app_state).await;
}

/// Get local IP address
fn local_ip() -> Option<String> {
    use std::net::UdpSocket;
    
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip().to_string())
}
