//! HTTP/WebSocket Server for VIBE-ON!
//!
//! Provides:
//! - REST API for library browsing, playback state, cover art
//! - WebSocket for real-time control and state updates
//! - mDNS advertisement for automatic discovery

pub mod routes;
pub mod websocket;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::get,
    Router,
};
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::{Any, CorsLayer};

use tauri::{AppHandle, Manager};

use self::routes::*;
use self::websocket::*;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Port to listen on
    pub port: u16,
    /// Server name for mDNS
    pub server_name: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 5000,
            server_name: crate::p2p::get_device_name(),
        }
    }
}

/// Shared server state
pub struct ServerState {
    /// Tauri app handle to access the real AppState
    pub app_handle: AppHandle,
    /// Broadcast channel for events to WebSocket clients
    pub event_tx: broadcast::Sender<ServerEvent>,
    /// Connected WebSocket clients
    pub clients: RwLock<Vec<ConnectedClient>>,
    /// Active output device ("desktop" or "mobile")
    pub active_output: RwLock<String>,
    /// Server configuration
    pub config: ServerConfig,
}

impl ServerState {
    pub fn new(app_handle: AppHandle, config: ServerConfig) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            app_handle,
            event_tx,
            clients: RwLock::new(Vec::new()),
            active_output: RwLock::new("desktop".to_string()),
            config,
        }
    }
    
    /// Get the app state from the Tauri app handle
    pub fn app_state(&self) -> tauri::State<'_, crate::AppState> {
        self.app_handle.state::<crate::AppState>()
    }
    
    /// Broadcast an event to all connected clients
    pub fn broadcast(&self, event: ServerEvent) {
        let _ = self.event_tx.send(event);
    }
}

/// Events broadcast to WebSocket clients
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ServerEvent {
    /// Playback state changed
    MediaSession {
        #[serde(rename = "trackId")]
        track_id: String,
        title: String,
        artist: String,
        album: String,
        duration: f64,
        #[serde(rename = "coverUrl")]
        cover_url: Option<String>,
        #[serde(rename = "isPlaying")]
        is_playing: bool,
        position: f64,
        timestamp: u64,
    },
    /// Playback position update
    PositionUpdate {
        position: f64,
        timestamp: u64,
    },
    /// Volume/shuffle/repeat status
    Status {
        volume: f64,
        shuffle: bool,
        #[serde(rename = "repeatMode")]
        repeat_mode: String,
        output: String,
    },
    /// Queue updated
    QueueUpdate {
        tracks: Vec<TrackSummary>,
    },
    /// Lyrics available
    Lyrics {
        #[serde(rename = "trackPath")]
        track_path: String,
        #[serde(rename = "hasSynced")]
        has_synced: bool,
        #[serde(rename = "syncedLyrics")]
        synced_lyrics: Option<String>,
        #[serde(rename = "plainLyrics")]
        plain_lyrics: Option<String>,
        instrumental: bool,
    },
    /// P2P handoff preparation
    HandoffPrepare {
        sample: u64,
        url: String,
    },
    /// Commit handoff (start playing)
    HandoffCommit,
    /// Stream stopped (returned to desktop)
    StreamStopped,
    /// Error occurred
    Error {
        message: String,
    },
    /// Pong response
    Pong,
    /// WebRTC Offer
    WebrtcOffer {
        from_peer_id: String,
        sdp: String,
    },
    /// WebRTC Answer
    WebrtcAnswer {
        target_peer_id: String, 
        sdp: String,
    },
    /// ICE Candidate
    IceCandidate {
        from_peer_id: String,
        candidate: String,
    },
}

/// Track summary for queue updates
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrackSummary {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    #[serde(rename = "durationSecs")]
    pub duration_secs: f64,
    #[serde(rename = "coverUrl")]
    pub cover_url: Option<String>,
}

/// Connected WebSocket client info
#[derive(Debug, Clone)]
pub struct ConnectedClient {
    pub id: String,
    pub name: String,
    pub connected_at: std::time::Instant,
}

/// Start the HTTP/WebSocket server
pub async fn start_server(
    app_handle: AppHandle,
    config: ServerConfig,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port = config.port;
    let server_state = Arc::new(ServerState::new(app_handle.clone(), config));
    
    // Spawn periodic status broadcast task (every 2 seconds)
    let broadcast_state = server_state.clone();
    let broadcast_handle = app_handle.clone();
    
    // Use a separate shutdown signal for the broadcast task
    let mut broadcast_shutdown = shutdown_rx.resubscribe();
    
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Only broadcast if there are connected clients
                    let clients = broadcast_state.clients.read().await;
                    if !clients.is_empty() {
                        drop(clients); // Release lock before calling send_current_status
                        
                        // --- Autoplay Logic ---
                        let app_state = broadcast_state.app_state();
                        let should_autoplay = {
                            if let Ok(player_guard) = app_state.player.lock() {
                                if let Some(ref player) = *player_guard {
                                    let status = player.get_status();
                                    // If stopped naturally (meaning sink empty in player thread)
                                    status.state == crate::audio::PlayerState::Stopped
                                } else { false }
                            } else { false }
                        };

                        if should_autoplay {
                            // Check if we have anything in queue to play next
                            let next_action = {
                                let queue = app_state.queue.lock().unwrap();
                                let mut index_guard = app_state.current_queue_index.lock().unwrap();
                                let repeat_mode = app_state.repeat_mode.lock().unwrap();
                                
                                if queue.is_empty() {
                                    None
                                } else {
                                    let mut next_idx = *index_guard + 1;
                                    let mut do_play = true;
                                    
                                    if next_idx >= queue.len() {
                                        if *repeat_mode == "all" {
                                            next_idx = 0;
                                        } else {
                                            do_play = false;
                                        }
                                    }
                                    
                                    if *repeat_mode == "one" {
                                        next_idx = *index_guard; // Keep same index
                                        do_play = true;
                                    }

                                    if do_play {
                                        *index_guard = next_idx;
                                        Some(queue[next_idx].path.clone())
                                    } else {
                                        None
                                    }
                                }
                            };

                            if let Some(path) = next_action {
                                println!("[Autoplay] Automatically playing next track: {}", path);
                                
                                // Trigger playback (silently on PC if mobile is active)
                                let is_mobile = {
                                    broadcast_state.active_output.read().await.as_str() == "mobile"
                                };
                                
                                if let Ok(mut player_guard) = app_state.player.lock() {
                                    if let Some(ref mut player) = *player_guard {
                                        if is_mobile {
                                            let _ = player.load_file(&path);
                                        } else {
                                            let _ = player.play_file(&path);
                                        }
                                    }
                                }
                                
                                // Broadcast the update so mobile knows to fetch new stream URL if needed
                                websocket::send_current_status_with_handle(&broadcast_state, &broadcast_handle).await;
                            }
                        } else {
                            websocket::send_current_status_with_handle(&broadcast_state, &broadcast_handle).await;
                        }
                    }
                }
                _ = broadcast_shutdown.recv() => {
                    println!("[Server] Stopping status broadcast task");
                    break;
                }
            }
        }
    });
    
    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(health_check))
        // API routes
        .route("/api/info", get(get_server_info))
        .route("/api/playback", get(get_playback_state))
        .route("/api/library", get(get_library))
        .route("/api/library/search", get(search_library))
        .route("/api/albums", get(get_albums))
        .route("/api/albums/:name/:artist", get(get_album_detail))
        .route("/api/artists", get(get_artists))
        .route("/api/artists/:name", get(get_artist_detail))
        .route("/api/lyrics/*path", get(get_lyrics))
        // Cover art
        .route("/cover/*path", get(get_cover))
        // Audio streaming
        .route("/stream/:path", get(stream_audio_file))
        .route("/stream", get(stream_audio))
        // WebSocket
        .route("/control", get(websocket_handler))
        // CORS
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(server_state.clone());
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("Starting VIBE-ON! server on http://{}", addr);
    println!("[Server] HTTP/WS listening on http://{}", addr);
    
    // Start mDNS advertisement
    let server_name = server_state.config.server_name.clone();
    
    // Use select to handle mDNS task with shutdown
    let mut mdns_shutdown = shutdown_rx.resubscribe();
    tokio::spawn(async move {
        tokio::select! {
            _ = advertise_mdns(&server_name, port) => {
                 log::error!("mDNS advertisement ended unexpectedly");
            }
            _ = mdns_shutdown.recv() => {
                println!("[Server] Stopping mDNS advertisement");
            }
        }
    });
    
    // Start server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.recv().await;
            println!("[Server] Graceful shutdown signal received");
        })
        .await?;
    
    Ok(())
}

/// Advertise the server via mDNS
async fn advertise_mdns(server_name: &str, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use mdns_sd::{ServiceDaemon, ServiceInfo};
    
    log::info!("mDNS: Advertising _vibe-on._tcp on port {}", port);
    
    // Create a daemon
    let mdns = ServiceDaemon::new()?;
    
    // Create a service info.
    // The service type must end with a period.
    let service_type = "_vibe-on._tcp.local.";
    let instance_name = server_name;
    
    // Get primary IPv4 address (exclude loopback and link-local)
    let ipv4_addr = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .find_map(|iface| {
            let addr = iface.addr.ip();
            // Only IPv4, not loopback, not link-local
            if addr.is_ipv4() && !addr.is_loopback() {
                Some(addr.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "".to_string());
    
    log::info!("mDNS: Using IPv4 address: {}", ipv4_addr);
    
    // Create service info with specific IPv4 address as hostname
    let service_info = ServiceInfo::new(
        service_type,
        instance_name,
        &format!("{}.local.", instance_name),
        &ipv4_addr, // Use IPv4 address directly
        port,
        &[("version", "1")][..]
    )?;
    
    // Register the service
    mdns.register(service_info)?;
    
    log::info!("mDNS: Service registered successfully with IPv4 address");
    
    // Keep the advertisement running
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
