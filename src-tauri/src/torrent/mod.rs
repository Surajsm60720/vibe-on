use librqbit::api::TorrentIdOrHash;
use librqbit::{AddTorrent, AddTorrentOptions, AddTorrentResponse, Session, SessionOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::fs as tokio_fs;

pub mod search;

// ============================================================================
// Constants
// ============================================================================

const STATE_FILE: &str = "vibe_torrents.json";

/// Audio file extensions we care about
const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "aac", "ogg", "m4a", "wma", "aiff", "alac", "opus",
];

/// Public trackers for better peer discovery - extensive list for maximum connectivity
const PUBLIC_TRACKERS: &[&str] = &[
    // Most reliable UDP trackers
    "udp://tracker.opentrackr.org:1337/announce",
    "udp://open.stealth.si:80/announce",
    "udp://tracker.openbittorrent.com:6969/announce",
    "udp://exodus.desync.com:6969/announce",
    "udp://tracker.torrent.eu.org:451/announce",
    "udp://tracker.moeking.me:6969/announce",
    "udp://explodie.org:6969/announce",
    "udp://opentracker.i2p.rocks:6969/announce",
    "udp://tracker.tiny-vps.com:6969/announce",
    "udp://p4p.arenabg.com:1337/announce",
    "udp://tracker.theoks.net:6969/announce",
    "udp://open.demonii.com:1337/announce",
    "udp://tracker.pomf.se:80/announce",
    "udp://tracker.coppersurfer.tk:6969/announce",
    "udp://tracker.leechers-paradise.org:6969/announce",
    "udp://tracker.internetwarriors.net:1337/announce",
    "udp://tracker.cyberia.is:6969/announce",
    "udp://9.rarbg.com:2810/announce",
    "udp://9.rarbg.me:2710/announce",
    "udp://9.rarbg.to:2710/announce",
    "udp://tracker.ds.is:6969/announce",
    "udp://retracker.lanta-net.ru:2710/announce",
    "udp://tracker.dler.org:6969/announce",
    "udp://tracker.0x.tf:6969/announce",
    "udp://bt.oiyo.tk:6969/announce",
    "udp://tracker.monitorit4.me:6969/announce",
    "udp://tracker.zer0day.to:1337/announce",
    "udp://tracker.filemail.com:6969/announce",
    "udp://ipv4.tracker.harry.lu:80/announce",
    "udp://tracker.justseed.it:1337/announce",
    // Reliable HTTP/HTTPS trackers as fallback
    "http://tracker.openbittorrent.com:80/announce",
    "http://tracker.opentrackr.org:1337/announce",
    "https://tracker.gbitt.info:443/announce",
    "https://tracker.imgoingto.icu:443/announce",
    "https://tracker.lilithraws.org:443/announce",
    "http://tracker.ipv6tracker.ru:80/announce",
    "http://nyaa.tracker.wf:7777/announce",
    "http://tracker.files.fm:6969/announce",
];

// ============================================================================
// Data Types
// ============================================================================

/// File information from a torrent
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TorrentFile {
    pub index: usize,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_audio: bool,
}

/// Status of an active torrent download
#[derive(Serialize, Clone, Debug)]
pub struct TorrentStatus {
    pub id: usize,
    pub name: String,
    pub progress: f64,
    pub download_speed: f64,
    pub upload_speed: f64,
    pub state: String,
    pub total_size: u64,
    pub downloaded_size: u64,
    pub peers_connected: u32,
    pub error: Option<String>,
}

/// Persisted torrent for saving/loading state
#[derive(Serialize, Deserialize, Clone, Debug)]
struct PersistedTorrent {
    magnet: Option<String>,
    file_bytes: Option<Vec<u8>>,
    output_folder: String,
    selected_files: Option<Vec<usize>>,
    info_hash: String,
    name: String,
}

/// Metadata about a torrent we're tracking
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct TorrentMetadata {
    id: usize,
    name: String,
    info_hash: String,
    magnet: Option<String>,
    file_bytes: Option<Vec<u8>>,
    output_folder: String,
    selected_files: Option<Vec<usize>>,
}

// ============================================================================
// Torrent Manager
// ============================================================================

#[derive(Clone)]
pub struct TorrentManager {
    session: Arc<Session>,
    /// The default download directory
    pub download_dir: PathBuf,
    /// Map of torrent ID -> metadata
    torrents: Arc<RwLock<HashMap<usize, TorrentMetadata>>>,
}

impl TorrentManager {
    /// Create a new TorrentManager with the given download directory
    pub async fn new(download_dir: PathBuf) -> Result<Self, String> {
        println!("[Torrent] Initializing with download_dir: {:?}", download_dir);

        // Ensure download directory exists
        if !download_dir.exists() {
            tokio_fs::create_dir_all(&download_dir)
                .await
                .map_err(|e| format!("Failed to create download directory: {}", e))?;
        }

        // Configure session options for maximum connectivity and speed
        let peer_opts = librqbit::PeerConnectionOptions {
            connect_timeout: Some(Duration::from_secs(20)),       // Faster timeout
            read_write_timeout: Some(Duration::from_secs(60)),
            keep_alive_interval: Some(Duration::from_secs(30)),
        };

        let options = SessionOptions {
            disable_dht: false,                    // DHT enabled for peer discovery
            disable_dht_persistence: true,
            persistence: None,
            listen_port_range: Some(6881..6999),   // Wide port range
            enable_upnp_port_forwarding: true,     // NAT traversal
            peer_opts: Some(peer_opts),
            
            // Optimize for more connections and faster downloads
            default_storage_factory: None,
            ..Default::default()
        };

        let session = Session::new_with_opts(download_dir.clone(), options)
            .await
            .map_err(|e| format!("Failed to create session: {}", e))?;

        println!("[Torrent] Session created successfully");

        let manager = Self {
            session,
            download_dir: download_dir.clone(),
            torrents: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load persisted state
        if let Err(e) = manager.load_state().await {
            eprintln!("[Torrent] Warning: Failed to load state: {}", e);
        }

        Ok(manager)
    }

    // ========================================================================
    // Public API
    // ========================================================================

    /// Inspect a magnet link to get file list without starting download
    /// Returns file information for user to select which files to download
    pub async fn inspect_magnet(&self, magnet: &str) -> Result<(String, Vec<TorrentFile>), String> {
        println!("[Torrent] Inspecting magnet: {}...", &magnet[..magnet.len().min(60)]);

        // Collect trackers for better metadata fetching
        let trackers: Vec<String> = PUBLIC_TRACKERS.iter().map(|s| s.to_string()).collect();

        // Add torrent in list_only mode to get metadata without downloading
        let opts = AddTorrentOptions {
            list_only: true,  // Just get file list, don't start download
            trackers: Some(trackers),
            ..Default::default()
        };

        let handle = self
            .session
            .add_torrent(AddTorrent::from_url(magnet), Some(opts))
            .await
            .map_err(|e| format!("Failed to add torrent for inspection: {}", e))?;

        // For list_only, we get the files directly from ListOnlyResponse
        let (name, files): (String, Vec<TorrentFile>) = match handle {
            AddTorrentResponse::ListOnly(list_only) => {
                let name = list_only.info.name
                    .as_ref()
                    .map(|b| String::from_utf8_lossy(b.as_ref()).to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                
                let file_details = list_only.info.iter_file_details()
                    .map_err(|e| format!("Failed to iterate file details: {}", e))?;
                
                let mut files = Vec::new();
                for (idx, fd) in file_details.enumerate() {
                    let path = fd.filename.to_string()
                        .map_err(|e| format!("Failed to get filename: {}", e))?;
                    let fname = std::path::Path::new(&path)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.clone());
                    let extension = std::path::Path::new(&path)
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    let is_audio = AUDIO_EXTENSIONS.contains(&extension.as_str());
                    
                    files.push(TorrentFile {
                        index: idx,
                        name: fname,
                        path,
                        size: fd.len,
                        is_audio,
                    });
                }
                (name, files)
            }
            AddTorrentResponse::Added(id, managed) | AddTorrentResponse::AlreadyManaged(id, managed) => {
                // Fallback: if we got Added response, wait for metadata then delete
                println!("[Torrent] Got Added response instead of ListOnly, waiting for metadata...");
                let files = self.wait_for_metadata(id, Duration::from_secs(120)).await?;
                let name = managed.name().unwrap_or_else(|| "Unknown".to_string());
                let _ = self.session.delete(TorrentIdOrHash::Id(id), false).await;
                (name, files)
            }
        };

        // Filter to show audio files first
        let mut sorted_files = files;
        sorted_files.sort_by(|a, b| {
            match (a.is_audio, b.is_audio) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.index.cmp(&b.index),
            }
        });

        Ok((name, sorted_files))
    }

    /// Inspect a .torrent file to get file list
    pub async fn inspect_torrent_file(&self, data: Vec<u8>) -> Result<(String, Vec<TorrentFile>), String> {
        println!("[Torrent] Inspecting torrent file ({} bytes)", data.len());

        // Use list_only mode for .torrent files - no need to add to session
        let opts = AddTorrentOptions {
            list_only: true,
            ..Default::default()
        };

        let handle = self
            .session
            .add_torrent(AddTorrent::TorrentFileBytes(data.into()), Some(opts))
            .await
            .map_err(|e| format!("Failed to parse torrent file: {}", e))?;

        let (name, files): (String, Vec<TorrentFile>) = match handle {
            AddTorrentResponse::ListOnly(list_only) => {
                let name = list_only.info.name
                    .as_ref()
                    .map(|b| String::from_utf8_lossy(b.as_ref()).to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                
                let file_details = list_only.info.iter_file_details()
                    .map_err(|e| format!("Failed to iterate file details: {}", e))?;
                
                let mut files = Vec::new();
                for (idx, fd) in file_details.enumerate() {
                    let path = fd.filename.to_string()
                        .map_err(|e| format!("Failed to get filename: {}", e))?;
                    let fname = std::path::Path::new(&path)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.clone());
                    let extension = std::path::Path::new(&path)
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    let is_audio = AUDIO_EXTENSIONS.contains(&extension.as_str());
                    
                    files.push(TorrentFile {
                        index: idx,
                        name: fname,
                        path,
                        size: fd.len,
                        is_audio,
                    });
                }
                (name, files)
            }
            AddTorrentResponse::Added(id, managed) | AddTorrentResponse::AlreadyManaged(id, managed) => {
                // Fallback if list_only didn't work
                let files = self.wait_for_metadata(id, Duration::from_secs(5)).await?;
                let name = managed.name().unwrap_or_else(|| "Unknown".to_string());
                let _ = self.session.delete(TorrentIdOrHash::Id(id), false).await;
                (name, files)
            }
        };

        let mut sorted_files = files;
        sorted_files.sort_by(|a, b| {
            match (a.is_audio, b.is_audio) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.index.cmp(&b.index),
            }
        });

        Ok((name, sorted_files))
    }

    /// Add a torrent and start downloading
    /// selected_files: MUST be provided as file indices to download. Pass None only to download ALL files.
    pub async fn add_torrent(
        &self,
        magnet: Option<String>,
        file_bytes: Option<Vec<u8>>,
        output_folder: String,
        selected_files: Option<Vec<usize>>,
    ) -> Result<usize, String> {
        println!("[Torrent] Adding torrent to: {}", output_folder);
        println!("[Torrent] Selected files: {:?}", selected_files);

        // Ensure output folder exists
        let output_path = PathBuf::from(&output_folder);
        if !output_path.exists() {
            tokio_fs::create_dir_all(&output_path)
                .await
                .map_err(|e| format!("Failed to create output folder: {}", e))?;
        }

        // Build the add source - use raw magnet since we'll inject trackers via opts.trackers
        let add_source = if let Some(ref m) = magnet {
            AddTorrent::from_url(m)
        } else if let Some(ref bytes) = file_bytes {
            AddTorrent::TorrentFileBytes(bytes.clone().into())
        } else {
            return Err("Either magnet or file_bytes must be provided".to_string());
        };

        // Collect trackers as Vec<String> for the API
        let trackers: Vec<String> = PUBLIC_TRACKERS.iter().map(|s| s.to_string()).collect();

        // Configure download options
        // IMPORTANT: only_files is respected - pass the exact file indices you want
        let opts = AddTorrentOptions {
            output_folder: Some(output_folder.clone()),
            overwrite: true,
            only_files: selected_files.clone(),
            trackers: Some(trackers), // Inject trackers via API instead of URL manipulation
            ..Default::default()
        };

        let handle = self
            .session
            .add_torrent(add_source, Some(opts))
            .await
            .map_err(|e| format!("Failed to add torrent: {}", e))?;

        let (id, managed) = match handle {
            AddTorrentResponse::Added(id, m) => {
                println!("[Torrent] Added new torrent with ID: {}", id);
                (id, m)
            }
            AddTorrentResponse::AlreadyManaged(id, m) => {
                println!("[Torrent] Torrent already managed with ID: {}", id);
                (id, m)
            }
            AddTorrentResponse::ListOnly(_) => return Err("Unexpected ListOnly response".to_string()),
        };

        // Get torrent name and info hash
        let name = managed.name().unwrap_or_else(|| format!("Torrent {}", id));
        let info_hash_bytes = managed.info_hash();
        let info_hash = info_hash_bytes.0.iter().map(|b| format!("{:02x}", b)).collect::<String>();

        // Store metadata
        {
            let mut torrents = self.torrents.write().unwrap();
            torrents.insert(id, TorrentMetadata {
                id,
                name: name.clone(),
                info_hash,
                magnet,
                file_bytes,
                output_folder,
                selected_files,
            });
        }

        // Save state
        if let Err(e) = self.save_state().await {
            eprintln!("[Torrent] Warning: Failed to save state: {}", e);
        }

        println!("[Torrent] Started downloading: {}", name);
        Ok(id)
    }

    /// Get status of all active torrents
    pub fn get_all_status(&self) -> Vec<TorrentStatus> {
        self.session.with_torrents(|torrents| {
            torrents
                .map(|(id, torrent)| {
                    let stats = torrent.stats();
                    
                    // Get the actual torrent name
                    let name = torrent.name().unwrap_or_else(|| {
                        // Fallback to stored metadata
                        self.torrents
                            .read()
                            .ok()
                            .and_then(|t| t.get(&id).map(|m| m.name.clone()))
                            .unwrap_or_else(|| format!("Torrent {}", id))
                    });

                    let progress = if stats.total_bytes > 0 {
                        stats.progress_bytes as f64 / stats.total_bytes as f64
                    } else {
                        0.0
                    };

                    let (state, download_speed, upload_speed, peers) = if stats.finished {
                        ("Finished".to_string(), 0.0, 0.0, 0)
                    } else if let Some(ref live) = stats.live {
                        let peer_count = live.snapshot.peer_stats.live as u32;
                        
                        // Log peer discovery status for debugging
                        if peer_count == 0 {
                            println!("[Torrent] ID {} - No peers yet. Queued: {}, Connecting: {}, Seen: {}", 
                                id, 
                                live.snapshot.peer_stats.queued,
                                live.snapshot.peer_stats.connecting,
                                live.snapshot.peer_stats.seen
                            );
                        }
                        
                        (
                            "Downloading".to_string(),
                            live.download_speed.mbps * 1024.0 * 1024.0, // Convert to bytes/sec
                            live.upload_speed.mbps * 1024.0 * 1024.0,
                            peer_count,
                        )
                    } else {
                        ("Paused".to_string(), 0.0, 0.0, 0)
                    };

                    TorrentStatus {
                        id,
                        name,
                        progress,
                        download_speed,
                        upload_speed,
                        state,
                        total_size: stats.total_bytes,
                        downloaded_size: stats.progress_bytes,
                        peers_connected: peers,
                        error: stats.error.clone(),
                    }
                })
                .collect()
        })
    }

    /// Pause a torrent
    pub async fn pause(&self, id: usize) -> Result<(), String> {
        let handle = self.get_handle(id).ok_or("Torrent not found")?;
        self.session.pause(&handle).await.map_err(|e| e.to_string())?;
        self.save_state().await?;
        println!("[Torrent] Paused torrent {}", id);
        Ok(())
    }

    /// Resume a torrent
    pub async fn resume(&self, id: usize) -> Result<(), String> {
        let handle = self.get_handle(id).ok_or("Torrent not found")?;
        
        // The session.unpause method requires Arc<Session>
        let session = Arc::clone(&self.session);
        session.unpause(&handle).await.map_err(|e| e.to_string())?;
        
        self.save_state().await?;
        println!("[Torrent] Resumed torrent {}", id);
        Ok(())
    }

    /// Delete a torrent
    pub async fn delete(&self, id: usize, delete_files: bool) -> Result<(), String> {
        println!("[Torrent] Deleting torrent {} (delete_files: {})", id, delete_files);

        // Delete from session
        self.session
            .delete(TorrentIdOrHash::Id(id), delete_files)
            .await
            .map_err(|e| format!("Failed to delete torrent: {}", e))?;

        // Remove from our tracking
        {
            let mut torrents = self.torrents.write().unwrap();
            torrents.remove(&id);
        }

        self.save_state().await?;
        Ok(())
    }

    // ========================================================================
    // Private Helpers
    // ========================================================================

    fn get_handle(&self, id: usize) -> Option<Arc<librqbit::ManagedTorrent>> {
        self.session.with_torrents(|torrents| {
            for (tid, handle) in torrents {
                if tid == id {
                    return Some(handle.clone());
                }
            }
            None
        })
    }

    async fn wait_for_metadata(&self, id: usize, timeout: Duration) -> Result<Vec<TorrentFile>, String> {
        let start = std::time::Instant::now();

        loop {
            let files_opt = self.session.with_torrents(|torrents| {
                for (tid, handle) in torrents {
                    if tid == id {
                        if let Ok(files) = handle.with_metadata(|metadata| {
                            metadata
                                .file_infos
                                .iter()
                                .enumerate()
                                .map(|(idx, file_info)| {
                                    let path = file_info.relative_filename.to_string_lossy().into_owned();
                                    let name = file_info
                                        .relative_filename
                                        .file_name()
                                        .map(|n| n.to_string_lossy().into_owned())
                                        .unwrap_or_else(|| path.clone());
                                    
                                    let extension = file_info
                                        .relative_filename
                                        .extension()
                                        .map(|e| e.to_string_lossy().to_lowercase())
                                        .unwrap_or_default();
                                    
                                    let is_audio = AUDIO_EXTENSIONS.contains(&extension.as_str());

                                    TorrentFile {
                                        index: idx,
                                        name,
                                        path,
                                        size: file_info.len,
                                        is_audio,
                                    }
                                })
                                .collect::<Vec<_>>()
                        }) {
                            return Some(files);
                        }
                        return None;
                    }
                }
                None
            });

            if let Some(files) = files_opt {
                return Ok(files);
            }

            if start.elapsed() > timeout {
                return Err("Timeout waiting for torrent metadata".to_string());
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    async fn save_state(&self) -> Result<(), String> {
        let state_path = self.download_dir.join(STATE_FILE);

        let persisted: Vec<PersistedTorrent> = {
            let torrents = self.torrents.read().unwrap();
            torrents
                .values()
                .map(|m| PersistedTorrent {
                    magnet: m.magnet.clone(),
                    file_bytes: m.file_bytes.clone(),
                    output_folder: m.output_folder.clone(),
                    selected_files: m.selected_files.clone(),
                    info_hash: m.info_hash.clone(),
                    name: m.name.clone(),
                })
                .collect()
        };

        let json = serde_json::to_string_pretty(&persisted)
            .map_err(|e| format!("Failed to serialize state: {}", e))?;

        tokio_fs::write(&state_path, json)
            .await
            .map_err(|e| format!("Failed to write state file: {}", e))?;

        Ok(())
    }

    async fn load_state(&self) -> Result<(), String> {
        let state_path = self.download_dir.join(STATE_FILE);

        if !state_path.exists() {
            return Ok(());
        }

        let json = tokio_fs::read_to_string(&state_path)
            .await
            .map_err(|e| format!("Failed to read state file: {}", e))?;

        let persisted: Vec<PersistedTorrent> = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse state file: {}", e))?;

        println!("[Torrent] Loading {} persisted torrents", persisted.len());
        
        // Prepare trackers for all torrents
        let trackers: Vec<String> = PUBLIC_TRACKERS.iter().map(|s| s.to_string()).collect();

        for p in persisted {
            let add_source = if let Some(ref m) = p.magnet {
                AddTorrent::from_url(m)
            } else if let Some(ref bytes) = p.file_bytes {
                AddTorrent::TorrentFileBytes(bytes.clone().into())
            } else {
                eprintln!("[Torrent] Skipping persisted torrent with no source: {}", p.name);
                continue;
            };

            let opts = AddTorrentOptions {
                output_folder: Some(p.output_folder.clone()),
                overwrite: true,
                only_files: p.selected_files.clone(),
                trackers: Some(trackers.clone()), // Inject trackers via API
                ..Default::default()
            };

            match self.session.add_torrent(add_source, Some(opts)).await {
                Ok(response) => {
                    let id = match response {
                        AddTorrentResponse::Added(id, _) => id,
                        AddTorrentResponse::AlreadyManaged(id, _) => id,
                        _ => continue,
                    };

                    let mut torrents = self.torrents.write().unwrap();
                    torrents.insert(id, TorrentMetadata {
                        id,
                        name: p.name.clone(),
                        info_hash: p.info_hash,
                        magnet: p.magnet,
                        file_bytes: p.file_bytes,
                        output_folder: p.output_folder,
                        selected_files: p.selected_files,
                    });

                    println!("[Torrent] Restored: {}", p.name);
                }
                Err(e) => {
                    eprintln!("[Torrent] Failed to restore {}: {}", p.name, e);
                }
            }
        }

        Ok(())
    }
}
