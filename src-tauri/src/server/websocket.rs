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
    /// Request library
    GetLibrary,
    /// Start mobile playback (client-initiated)
    StartMobilePlayback,
    /// Stop mobile playback (client-initiated)
    StopMobilePlayback,
    /// Mobile playback position update
    MobilePositionUpdate { position_secs: f64 },
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
    /// Library data
    #[serde(rename_all = "camelCase")]
    Library {
        tracks: Vec<super::routes::TrackDetail>,
    },
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
    log::info!("🔌 New WebSocket connection established");
    println!("[WebSocket] New connection!");
    
    let (mut sender, mut receiver) = socket.split();
    
    // Subscribe to broadcast events
    let mut event_rx = state.event_tx.subscribe();
    
    // Generate client ID
    let client_id = uuid::Uuid::new_v4().to_string();
    log::info!("🔌 WebSocket client ID assigned: {}", client_id);
    println!("[WebSocket] Client ID: {}", client_id);
    let _client_id_clone = client_id.clone();
    let client_id_for_cleanup = client_id.clone();
    let app_handle = state.app_handle.clone();
    
    log::info!("📱 New WebSocket connection accepted (ID: {})", client_id);
    println!("[WebSocket] New client connected: {}", client_id);
    
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
                             log::debug!("📱 Sending reply to client: {:?}", msg);
                             let json = serde_json::to_string(&msg).unwrap();
                             if sender.send(Message::Text(json.into())).await.is_err() {
                                 log::warn!("❌ Failed to send reply to client");
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
                log::debug!("📱 WebSocket message received ({}): {}", client_id, text);
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        log::info!("📱 Parsed ClientMessage: {:?}", client_msg);
                        handle_client_message(&state, &client_id, client_msg, &reply_tx).await;
                    }
                    Err(e) => {
                        log::warn!("❌ Invalid WebSocket message: {}", e);
                        state.broadcast(ServerEvent::Error {
                            message: format!("Invalid message format: {}", e),
                        });
                    }
                }
            }
            Message::Close(_) => {
                log::info!("📱 WebSocket close received ({})", client_id);
                break;
            }
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
            log::info!("📱 Mobile HELLO received from: {} (ID: {})", client_name, client_id);
            
            // Add client to list
            let client = ConnectedClient {
                id: client_id.to_string(),
                name: client_name.clone(),
                connected_at: std::time::Instant::now(),
            };
            state.clients.write().await.push(client);
            
            log::info!("📱 Emitting mobile_client_connected event to frontend");
            println!("[WebSocket] Emitting mobile_client_connected: {} ({})", client_name, client_id);
            
            // Emit connection event to frontend
            let emit_result = state.app_handle.emit("mobile_client_connected", serde_json::json!({
                "client_id": client_id,
                "client_name": client_name,
            }));
            
            match emit_result {
                Ok(_) => log::info!("✅ mobile_client_connected event emitted successfully"),
                Err(e) => log::error!("❌ Failed to emit mobile_client_connected: {}", e),
            }
            log::info!("Mobile client connected: {} ({})", client_name, client_id);
            
            // Send Connected event to confirm handshake
            let _ = reply_tx.send(ServerMessage::Connected { 
                client_id: client_id.to_string() 
            }).await;
            log::info!("✅ Sent Connected acknowledgment to mobile");

            // Send current status immediately
            send_current_status_internal(state, &app_state, &reply_tx).await;
        }
        
        ClientMessage::GetStatus => {
            log::info!("📱 GetStatus request from mobile ({})", client_id);
            send_current_status_internal(state, &app_state, &reply_tx).await;
        }
        
        ClientMessage::Play => {
            log::info!("📱 Play command from mobile ({})", client_id);
            {
                if let Ok(mut player_guard) = app_state.player.lock() {
                    if let Some(ref mut player) = *player_guard {
                        player.resume();
                    }
                }
            }
            // Send acknowledgment
            let _ = reply_tx.send(ServerMessage::Error {
                message: "ok:play".to_string(),
            }).await;
            send_current_status_internal(state, &app_state, &reply_tx).await;
        }
        
        ClientMessage::Pause | ClientMessage::Stop => {
            log::info!("📱 Pause/Stop command from mobile ({})", client_id);
            {
                if let Ok(mut player_guard) = app_state.player.lock() {
                    if let Some(ref mut player) = *player_guard {
                        let _ = player.pause();
                    }
                }
            }
            // Send acknowledgment
            let _ = reply_tx.send(ServerMessage::Error {
                message: "ok:pause".to_string(),
            }).await;
            send_current_status_internal(state, &app_state, &reply_tx).await;
        }
        
        ClientMessage::Resume => {
            log::info!("📱 Resume command from mobile ({})", client_id);
            {
                if let Ok(mut player_guard) = app_state.player.lock() {
                    if let Some(ref mut player) = *player_guard {
                        let _ = player.resume();
                    }
                }
            }
            // Send acknowledgment
            let _ = reply_tx.send(ServerMessage::Error {
                message: "ok:resume".to_string(),
            }).await;
            send_current_status_internal(state, &app_state, &reply_tx).await;
        }
        
        ClientMessage::Next => {
            log::info!("📱 Next track command from mobile ({})", client_id);
            
            // Get current track path and next track path (must be done before await)
            let next_track = {
                let current_track = {
                    if let Ok(player_guard) = app_state.player.lock() {
                        if let Some(ref player) = *player_guard {
                            player.get_status().track.map(|t| t.path)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                
                // Get all tracks and find next one
                if let Ok(db_guard) = app_state.db.lock() {
                    if let Some(ref db) = *db_guard {
                        if let Ok(all_tracks) = db.get_all_tracks() {
                            if !all_tracks.is_empty() {
                                if let Some(current_path) = &current_track {
                                    let current_idx = all_tracks.iter().position(|t| &t.path == current_path);
                                    if let Some(idx) = current_idx {
                                        if idx + 1 < all_tracks.len() {
                                            Some(all_tracks[idx + 1].path.clone())
                                        } else {
                                            Some(all_tracks[0].path.clone())
                                        }
                                    } else {
                                        Some(all_tracks[0].path.clone())
                                    }
                                } else {
                                    Some(all_tracks[0].path.clone())
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            
            if let Some(next_track_path) = next_track {
                // Check if we need to stream to mobile
                let is_mobile = {
                    state.active_output.read().await.as_str() == "mobile"
                };

                // Play or load the next track
                {
                    if let Ok(mut player_guard) = app_state.player.lock() {
                        if let Some(ref player) = *player_guard {
                            if is_mobile {
                                let _ = player.load_file(&next_track_path);
                                log::info!("📱 Loading (mobile) next track: {}", next_track_path);
                            } else {
                                let _ = player.play_file(&next_track_path);
                                log::info!("▶️ Playing next track: {}", next_track_path);
                            }
                        }
                    }
                }
                
                // Send acknowledgment
                let _ = reply_tx.send(ServerMessage::Error {
                    message: "ok:next".to_string(),
                }).await;
                
                if is_mobile {
                    // Send stream URL to mobile
                    let (track_path, position, stream_url) = {
                        let player_guard = app_state.player.lock().ok();
                        match player_guard.as_ref().and_then(|p| p.as_ref()) {
                            Some(player) => {
                                let status = player.get_status();
                                match status.track {
                                    Some(track) => {
                                        let position = 0.0; // Start from beginning for new track
                                        let local_ip = local_ip().unwrap_or("127.0.0.1".to_string());
                                        let port = state.config.port;
                                        let encoded_path = urlencoding::encode(&track.path).to_string();
                                        let url = format!("http://{}:{}/stream/{}", local_ip, port, encoded_path);
                                        (Some(track.path), position, url)
                                    }
                                    None => (None, 0.0, String::new())
                                }
                            }
                            None => (None, 0.0, String::new())
                        }
                    };
                    
                    if track_path.is_some() {
                        log::info!("🎵 Sending next track stream via Handoff: {}", stream_url);
                        let _ = reply_tx.send(ServerMessage::HandoffPrepare { 
                            sample: 0,
                            url: stream_url 
                        }).await;
                    }
                }

                send_current_status_internal(state, &app_state, &reply_tx).await;
            } else {
                log::error!("❌ Could not play next track - no tracks available");
                let _ = reply_tx.send(ServerMessage::Error {
                    message: "No tracks available".to_string(),
                }).await;
            }
        }
        
        ClientMessage::Previous => {
            log::info!("📱 Previous track command from mobile ({})", client_id);
            
            // Get current track path and previous track path (must be done before await)
            let prev_track = {
                let current_track = {
                    if let Ok(player_guard) = app_state.player.lock() {
                        if let Some(ref player) = *player_guard {
                            player.get_status().track.map(|t| t.path)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                
                // Get all tracks and find previous one
                if let Ok(db_guard) = app_state.db.lock() {
                    if let Some(ref db) = *db_guard {
                        if let Ok(all_tracks) = db.get_all_tracks() {
                            if !all_tracks.is_empty() {
                                if let Some(current_path) = &current_track {
                                    let current_idx = all_tracks.iter().position(|t| &t.path == current_path);
                                    if let Some(idx) = current_idx {
                                        if idx > 0 {
                                            Some(all_tracks[idx - 1].path.clone())
                                        } else {
                                            Some(all_tracks[all_tracks.len() - 1].path.clone())
                                        }
                                    } else {
                                        Some(all_tracks[all_tracks.len() - 1].path.clone())
                                    }
                                } else {
                                    Some(all_tracks[all_tracks.len() - 1].path.clone())
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            
            if let Some(prev_track_path) = prev_track {
                // Check if we need to stream to mobile
                let is_mobile = {
                    state.active_output.read().await.as_str() == "mobile"
                };

                // Play or load the previous track
                {
                    if let Ok(mut player_guard) = app_state.player.lock() {
                        if let Some(ref player) = *player_guard {
                            if is_mobile {
                                let _ = player.load_file(&prev_track_path);
                                log::info!("📱 Loading (mobile) previous track: {}", prev_track_path);
                            } else {
                                let _ = player.play_file(&prev_track_path);
                                log::info!("⏮️ Playing previous track: {}", prev_track_path);
                            }
                        }
                    }
                }
                
                // Send acknowledgment
                let _ = reply_tx.send(ServerMessage::Error {
                    message: "ok:previous".to_string(),
                }).await;
                
                if is_mobile {
                    // Send stream URL to mobile
                    let (track_path, position, stream_url) = {
                        let player_guard = app_state.player.lock().ok();
                        match player_guard.as_ref().and_then(|p| p.as_ref()) {
                            Some(player) => {
                                let status = player.get_status();
                                match status.track {
                                    Some(track) => {
                                        let position = 0.0; // Start from beginning
                                        let local_ip = local_ip().unwrap_or("127.0.0.1".to_string());
                                        let port = state.config.port;
                                        let encoded_path = urlencoding::encode(&track.path).to_string();
                                        let url = format!("http://{}:{}/stream/{}", local_ip, port, encoded_path);
                                        (Some(track.path), position, url)
                                    }
                                    None => (None, 0.0, String::new())
                                }
                            }
                            None => (None, 0.0, String::new())
                        }
                    };
                    
                    if track_path.is_some() {
                        log::info!("🎵 Sending prev track stream via Handoff: {}", stream_url);
                        let _ = reply_tx.send(ServerMessage::HandoffPrepare { 
                            sample: 0,
                            url: stream_url 
                        }).await;
                    }
                }

                send_current_status_internal(state, &app_state, &reply_tx).await;
            } else {
                log::error!("❌ Could not play previous track - no tracks available");
                let _ = reply_tx.send(ServerMessage::Error {
                    message: "No tracks available".to_string(),
                }).await;
            }
        }
        
        ClientMessage::Seek { position_secs } => {
            log::info!("📱 Seek command from mobile ({}): {:.2}s", client_id, position_secs);
            {
                if let Ok(mut player_guard) = app_state.player.lock() {
                    if let Some(ref mut player) = *player_guard {
                        player.seek(position_secs);
                    }
                }
            }
            // Send acknowledgment
            let _ = reply_tx.send(ServerMessage::Error {
                message: "ok:seek".to_string(),
            }).await;
            send_current_status_internal(state, &app_state, &reply_tx).await;
        }
        
        ClientMessage::SetVolume { volume } => {
            log::info!("📱 SetVolume command from mobile ({}): {:.2}", client_id, volume);
            {
                if let Ok(mut player_guard) = app_state.player.lock() {
                    if let Some(ref mut player) = *player_guard {
                        player.set_volume(volume as f32);
                    }
                }
            }
            // Send acknowledgment
            let _ = reply_tx.send(ServerMessage::Error {
                message: "ok:setVolume".to_string(),
            }).await;
            send_current_status_internal(state, &app_state, &reply_tx).await;
        }
        
        ClientMessage::PlayTrack { path } => {
            log::info!("📱 PlayTrack command from mobile ({}): {}", client_id, path);
            
            let is_mobile = {
                state.active_output.read().await.as_str() == "mobile"
            };

            let play_result = {
                if let Ok(mut player_guard) = app_state.player.lock() {
                    if let Some(ref mut player) = *player_guard {
                        if is_mobile {
                            player.load_file(&path)
                        } else {
                            player.play_file(&path)
                        }
                    } else {
                        Err("No player available".to_string())
                    }
                } else {
                    Err("Failed to lock player".to_string())
                }
            };
            
            if let Err(e) = play_result {
                log::error!("Failed to play track: {}", e);
                let _ = reply_tx.send(ServerMessage::Error {
                    message: format!("Failed to play track: {}", e),
                }).await;
                return;
            }
            
            // Send acknowledgment
            let _ = reply_tx.send(ServerMessage::Error {
                message: "ok:playTrack".to_string(),
            }).await;

            if is_mobile {
                // Send stream URL to mobile
                let (track_path, position, stream_url) = {
                    let player_guard = app_state.player.lock().ok();
                    match player_guard.as_ref().and_then(|p| p.as_ref()) {
                        Some(player) => {
                            let status = player.get_status();
                            match status.track {
                                Some(track) => {
                                    let position = 0.0; // Start from beginning
                                    let local_ip = local_ip().unwrap_or("127.0.0.1".to_string());
                                    let port = state.config.port;
                                    let encoded_path = urlencoding::encode(&track.path).to_string();
                                    let url = format!("http://{}:{}/stream/{}", local_ip, port, encoded_path);
                                    (Some(track.path), position, url)
                                }
                                None => (None, 0.0, String::new())
                            }
                        }
                        None => (None, 0.0, String::new())
                    }
                };
                
                if track_path.is_some() {
                    log::info!("🎵 Sending track stream via Handoff: {}", stream_url);
                    let _ = reply_tx.send(ServerMessage::HandoffPrepare { 
                        sample: 0,
                        url: stream_url 
                    }).await;
                }
            }

            send_current_status_internal(state, &app_state, &reply_tx).await;
        }
        
        ClientMessage::RequestStreamToMobile => {
            // Get current track and prepare stream URL
            let (track_path, sample, url) = {
                let player_guard = app_state.player.lock().ok();
                match player_guard.as_ref().and_then(|p| p.as_ref()) {
                    Some(player) => {
                        let status = player.get_status();
                        match status.track {
                            Some(track) => {
                                let position = status.position_secs;
                                // Calculate sample position (assuming 44.1kHz)
                                let sample = (position * 44100.0) as u64;
                                let local_ip = local_ip().unwrap_or("127.0.0.1".to_string());
                                let port = state.config.port;
                                let encoded_path = urlencoding::encode(&track.path).to_string();
                                let url = format!("http://{}:{}/stream/{}", local_ip, port, encoded_path);
                                (Some(track.path), sample, url)
                            }
                            None => {
                                state.broadcast(ServerEvent::Error {
                                    message: "No track currently playing".to_string(),
                                });
                                return;
                            }
                        }
                    }
                    None => {
                        state.broadcast(ServerEvent::Error {
                            message: "Player not initialized".to_string(),
                        });
                        return;
                    }
                }
            };
            
            log::info!("[Stream] Mobile client requesting stream for: {:?}", track_path);
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
        
        ClientMessage::StartMobilePlayback => {
            log::info!("📱 StartMobilePlayback command from mobile ({})", client_id);
            
            // Set active output to mobile
            {
                let mut output = state.active_output.write().await;
                *output = "mobile".to_string();
            }
            log::info!("🔊 Active output set to: mobile");

            // Mute PC playback but keep it running for sync
            if let Ok(mut player_guard) = app_state.player.lock() {
                if let Some(ref mut player) = *player_guard {
                    let _ = player.set_mute(true);
                    // Ensure it is playing if it was paused? 
                    // Ideally we sync state first. For now, assume user pressed play on mobile.
                    // If mobile says "start playback", we essentially want the PC to "play silently".
                    let _ = player.resume(); 
                    log::info!("🔇 PC playback muted and resumed for mobile streaming");
                }
            }
            
            // Get current track info and send stream URL to mobile
            let (track_path, position, stream_url) = {
                let player_guard = app_state.player.lock().ok();
                match player_guard.as_ref().and_then(|p| p.as_ref()) {
                    Some(player) => {
                        let status = player.get_status();
                        match status.track {
                            Some(track) => {
                                let position = status.position_secs;
                                let local_ip = local_ip().unwrap_or("127.0.0.1".to_string());
                                let port = state.config.port;
                                let encoded_path = urlencoding::encode(&track.path).to_string();
                                let url = format!("http://{}:{}/stream/{}", local_ip, port, encoded_path);
                                (Some(track.path), position, url)
                            }
                            None => (None, 0.0, String::new())
                        }
                    }
                    None => (None, 0.0, String::new())
                }
            };
            
            if track_path.is_some() {
                log::info!("🎵 Sending stream URL to mobile: {}", stream_url);
                let _ = reply_tx.send(ServerMessage::HandoffPrepare { 
                    sample: (position * 44100.0) as u64,
                    url: stream_url 
                }).await;
                
                // Notify frontend that output changed
                let _ = state.app_handle.emit("output-changed", serde_json::json!({
                    "output": "mobile"
                }));
            } else {
                let _ = reply_tx.send(ServerMessage::Error {
                    message: "No track currently playing".to_string(),
                }).await;
            }
        }
        
        ClientMessage::StopMobilePlayback => {
            log::info!("📱 StopMobilePlayback command from mobile ({})", client_id);
            
            // Set active output to desktop
            {
                let mut output = state.active_output.write().await;
                *output = "desktop".to_string();
            }
            log::info!("🔊 Active output set to: desktop");

            // Unmute PC playback
            if let Ok(mut player_guard) = app_state.player.lock() {
                if let Some(ref mut player) = *player_guard {
                    let _ = player.set_mute(false);
                    log::info!("🔈 PC playback unmuted");
                }
            }
            state.broadcast(ServerEvent::StreamStopped);
            
            // Notify frontend that output changed
            let _ = state.app_handle.emit("output-changed", serde_json::json!({
                "output": "desktop"
            }));
        }

        ClientMessage::GetLyrics => {
            log::info!("📱 GetLyrics request from mobile ({})", client_id);
            
            // Get current track to fetch lyrics for
            let current_track = {
                if let Ok(player_guard) = app_state.player.lock() {
                    if let Some(ref player) = *player_guard {
                        player.get_status().track
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some(track) = current_track {
                let artist = track.artist.clone();
                let title = track.title.clone();
                let duration = track.duration_secs as u32;
                let path = track.path.clone();
                
                log::info!("📝 Fetching lyrics for: {} - {}", artist, title);
                
                // Fetch in background task to not block
                let reply_tx = reply_tx.clone();
                tokio::task::spawn_blocking(move || {
                    // Try local lrc first
                    println!("[Lyrics] Checking local file for: {}", path);
                    if let Some(lrc) = crate::lyrics_fetcher::find_local_lrc(&path) {
                        println!("[Lyrics] Found local file!");
                        let _ = reply_tx.blocking_send(ServerMessage::Lyrics {
                            track_path: path,
                            has_synced: lrc.synced_lyrics.is_some(),
                            synced_lyrics: lrc.synced_lyrics,
                            plain_lyrics: lrc.plain_lyrics,
                            instrumental: lrc.instrumental.unwrap_or(false),
                        });
                        return;
                    }
                    
                    // Fetch from API
                    match crate::lyrics_fetcher::fetch_lyrics(&artist, &title, duration, |_| {}) {
                        Ok(lyrics) => {
                             let _ = reply_tx.blocking_send(ServerMessage::Lyrics {
                                track_path: path,
                                has_synced: lyrics.synced_lyrics.is_some(),
                                synced_lyrics: lyrics.synced_lyrics,
                                plain_lyrics: lyrics.plain_lyrics,
                                instrumental: lyrics.instrumental.unwrap_or(false),
                            });
                        }
                        Err(e) => {
                            log::warn!("Failed to fetch lyrics: {}", e);
                            let _ = reply_tx.blocking_send(ServerMessage::Error {
                                message: "Lyrics not found".to_string(),
                            });
                        }
                    }
                });
            } else {
                 let _ = reply_tx.send(ServerMessage::Error {
                    message: "No track playing".to_string(),
                }).await;
            }
        }
        
        ClientMessage::MobilePositionUpdate { position_secs } => {
            log::debug!("📱 Mobile position update: {:.2}s", position_secs);
            // Sync position with mobile for consistent state
            // Could update PC's internal tracking if needed
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

        ClientMessage::GetLibrary => {
            log::info!("📱 GetLibrary request from mobile ({})", client_id);
            
            // Fetch all tracks from DB
            let tracks = if let Ok(db_guard) = app_state.db.lock() {
                if let Some(ref db) = *db_guard {
                    if let Ok(all_tracks) = db.get_all_tracks() {
                         all_tracks.into_iter().map(|t| super::routes::TrackDetail {
                            path: t.path.clone(),
                            title: t.title,
                            artist: t.artist,
                            album: t.album,
                            duration_secs: t.duration_secs,
                            disc_number: t.disc_number,
                            track_number: t.track_number,
                            cover_url: Some(format!("/cover/{}", urlencoding::encode(&t.path))),
                        }).collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };
            
            log::info!("📱 Sending library with {} tracks to mobile", tracks.len());
            let _ = reply_tx.send(ServerMessage::Library { tracks }).await;
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
async fn send_current_status_internal(
    state: &Arc<ServerState>,
    app_state: &tauri::State<'_, crate::AppState>,
    reply_tx: &tokio::sync::mpsc::Sender<ServerMessage>,
) {
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
    
    let active_output = state.active_output.read().await.clone();
    
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    // Send MediaSession message directly to client
    let media_msg = ServerMessage::MediaSession {
        track_id: track_id.clone(),
        title: title.clone(),
        artist: artist.clone(),
        album: album.clone(),
        duration,
        cover_url: cover_url.clone(),
        is_playing,
        position,
        timestamp,
    };
    let _ = reply_tx.send(media_msg).await;
    log::debug!("✅ Sent MediaSession to mobile client");
    
    // Also broadcast to all connected clients
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
    
    // Send Status message directly to client
    let status_msg = ServerMessage::Status {
        volume: volume as f64,
        shuffle: false,
        repeat_mode: "off".to_string(),
        output: active_output.clone(),
    };
    let _ = reply_tx.send(status_msg).await;
    log::debug!("✅ Sent Status to mobile client");

    // Emit event to Tauri frontend to refresh UI
    let _ = state.app_handle.emit("refresh-player-state", ());
    
    // Also broadcast to all connected clients
    state.broadcast(ServerEvent::Status {
        volume: volume as f64,
        shuffle: false,
        repeat_mode: "off".to_string(),
        output: active_output,
    });
}

/// Send current playback status (public, for periodic broadcasting from mod.rs)
pub async fn send_current_status_with_handle(state: &Arc<ServerState>, app_handle: &AppHandle) {
    let app_state: tauri::State<'_, crate::AppState> = app_handle.state();
    send_current_status_broadcast_only(state, &app_state).await;
}

/// Send current playback status (broadcast only, for periodic updates)
async fn send_current_status_broadcast_only(
    state: &Arc<ServerState>,
    app_state: &tauri::State<'_, crate::AppState>,
) {
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
    
    let active_output = state.active_output.read().await.clone();
    
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
        output: active_output,
    });
}

/// Get local IP address
fn local_ip() -> Option<String> {
    use std::net::UdpSocket;
    
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip().to_string())
}
