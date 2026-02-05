use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use lofty::prelude::*;
use lofty::probe::Probe;
use rodio::{Decoder, OutputStream, Sink, Source};

use super::equalizer::Equalizer;
use super::fft::{FftProcessor, VisualizerData, VisualizerTap};
use super::state::{PlayerState, PlayerStatus, TrackInfo};
use std::sync::{Mutex, RwLock};

/// Commands sent to the audio thread
pub enum AudioCommand {
    Play(String),
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
    Seek(f64),     // New command
    SetMute(bool), // Mute command
    Load(String),  // Load metadata only
    GetStatus(Sender<PlayerStatus>),
    Shutdown,
    SetEq(usize, f32), // band_index, gain_db
    SetSpeed(f32),
    SetReverb(f32, f32), // mix (0-1), decay (0-1)
    GetVisualizerData(Sender<VisualizerData>),
}

/// Thread-safe handle to the audio player
pub struct AudioPlayer {
    command_tx: Sender<AudioCommand>,
    _thread: JoinHandle<()>,
    eq_gains: Arc<Mutex<Vec<f32>>>,
    fft_processor: Arc<FftProcessor>,
}

impl AudioPlayer {
    /// Create a new audio player with a dedicated audio thread
    pub fn new() -> Result<Self, String> {
        let (command_tx, command_rx) = channel::<AudioCommand>();
        let (init_tx, init_rx) = std::sync::mpsc::sync_channel(0);

        // Initialize gains: 10 bands + Preamp + Balance + Width + Spares
        // 0-9: EQ
        // 10: Preamp (0dB)
        // 11: Balance (0.0)
        // 12: Stereo Width (1.0 default)
        let mut initial_gains = vec![0.0; 15];
        initial_gains[12] = 1.0;

        let eq_gains = Arc::new(Mutex::new(initial_gains));
        let eq_gains_clone = eq_gains.clone();

        // Create FFT processor for audio visualization
        let fft_processor = Arc::new(FftProcessor::new(44100)); // Will update sample rate on play
        let fft_buffer = fft_processor.get_buffer_handle();

        let thread = thread::spawn(move || {
            AudioThread::run(command_rx, init_tx, eq_gains_clone, fft_buffer);
        });

        // Wait for initialization to complete
        match init_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                command_tx,
                _thread: thread,
                eq_gains,
                fft_processor,
            }),
            Ok(Err(e)) => Err(format!("Audio initialization failed: {}", e)),
            Err(_) => Err("Audio thread panicked during initialization".to_string()),
        }
    }

    pub fn play_file(&self, path: &str) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::Play(path.to_string()))
            .map_err(|e| format!("Failed to send play command: {}", e))
    }

    pub fn load_file(&self, path: &str) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::Load(path.to_string()))
            .map_err(|e| format!("Failed to send load command: {}", e))
    }

    pub fn pause(&self) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::Pause)
            .map_err(|e| format!("Failed to send pause command: {}", e))
    }

    pub fn resume(&self) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::Resume)
            .map_err(|e| format!("Failed to send resume command: {}", e))
    }

    pub fn stop(&self) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::Stop)
            .map_err(|e| format!("Failed to send stop command: {}", e))
    }

    pub fn set_volume(&self, value: f32) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetVolume(value))
            .map_err(|e| format!("Failed to send volume command: {}", e))
    }

    pub fn seek(&self, seconds: f64) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::Seek(seconds))
            .map_err(|e| format!("Failed to send seek command: {}", e))
    }

    pub fn set_speed(&self, value: f32) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetSpeed(value))
            .map_err(|e| format!("Failed to send speed command: {}", e))
    }

    pub fn set_mute(&self, mute: bool) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetMute(mute))
            .map_err(|e| format!("Failed to send mute command: {}", e))
    }
    pub fn get_status(&self) -> PlayerStatus {
        let (tx, rx) = channel();
        if self.command_tx.send(AudioCommand::GetStatus(tx)).is_ok() {
            rx.recv().unwrap_or_default()
        } else {
            PlayerStatus::default()
        }
    }

    pub fn set_eq(&self, band: usize, gain: f32) -> Result<(), String> {
        // Update local state (shared memory)
        if let Ok(mut gains) = self.eq_gains.lock() {
            if band < gains.len() {
                gains[band] = gain;
            }
        }

        // Notify thread (though thread might poll, good to have explicit command if we move to push model later
        // OR simply to force wake up loop if needed, but here we just update memory and thread polls in Equalizer)
        // Actually, we need to send command if we want explicit event handling, but Equalizer source reads from the Arc.
        // Sending a command is useful if we want to log or ensure thread is alive.
        // For this impl, I'll send a command so the thread knows to wake/log,
        // AND mainly because the Equalizer struct inside the thread holds an Arc to the SAME Mutex?
        // Yes, we will pass the Arc to Equalizer.

        self.command_tx
            .send(AudioCommand::SetEq(band, gain))
            .map_err(|e| format!("Failed to send eq command: {}", e))
    }

    pub fn set_reverb(&self, mix: f32, decay: f32) -> Result<(), String> {
        // Update local state indices 13 (mix) and 14 (decay)
        if let Ok(mut gains) = self.eq_gains.lock() {
            if gains.len() >= 15 {
                gains[13] = mix.clamp(0.0, 1.0);
                gains[14] = decay.clamp(0.0, 1.0);
            }
        }

        self.command_tx
            .send(AudioCommand::SetReverb(mix, decay))
            .map_err(|e| format!("Failed to send reverb command: {}", e))
    }

    /// Get current visualizer data (frequency bins and waveform) for UI rendering
    pub fn get_visualizer_data(&self) -> VisualizerData {
        self.fft_processor.get_visualizer_data()
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        let _ = self.command_tx.send(AudioCommand::Shutdown);
    }
}

/// The actual audio thread that owns the non-Send types
struct AudioThread {
    sink: Option<Sink>,
    _stream: OutputStream,
    state: PlayerState,
    current_track: Option<TrackInfo>,
    current_path: Option<String>, // Store path for seek reload
    volume: f32,
    muted: bool,
    play_start_time: Option<Instant>,
    accumulated_time: f64,
    eq_gains: Arc<Mutex<Vec<f32>>>,
    fft_buffer: Arc<RwLock<super::fft::RingBuffer>>,
}

impl AudioThread {
    fn run(
        command_rx: Receiver<AudioCommand>,
        init_tx: std::sync::mpsc::SyncSender<Result<(), String>>,
        eq_gains: Arc<Mutex<Vec<f32>>>,
        fft_buffer: Arc<RwLock<super::fft::RingBuffer>>,
    ) {
        // Initialize audio output on this thread
        let (stream, stream_handle) = match OutputStream::try_default() {
            Ok(s) => s,
            Err(e) => {
                let err_msg = format!("Failed to open audio device: {}", e);
                eprintln!("{}", err_msg);
                let _ = init_tx.send(Err(err_msg));
                return;
            }
        };

        // Store stream_handle for creating sinks
        let stream_handle = Arc::new(stream_handle);

        // Signal success
        if let Err(e) = init_tx.send(Ok(())) {
            eprintln!("Failed to send init success: {}", e);
            return;
        }

        let mut audio = AudioThread {
            sink: None,
            _stream: stream,
            state: PlayerState::Stopped,
            current_track: None,
            current_path: None,
            volume: 1.0,
            muted: false,
            play_start_time: None,
            accumulated_time: 0.0,
            eq_gains,
            fft_buffer,
        };

        loop {
            // Use timeout to allow polling for track completion
            match command_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(AudioCommand::Play(path)) => {
                    audio.handle_play(&path, &stream_handle);
                }
                Ok(AudioCommand::Load(path)) => {
                    audio.handle_load(&path);
                }
                Ok(AudioCommand::Pause) => {
                    audio.handle_pause();
                }
                Ok(AudioCommand::Resume) => {
                    audio.handle_resume();
                }
                Ok(AudioCommand::Stop) => {
                    audio.handle_stop();
                }
                Ok(AudioCommand::SetVolume(value)) => {
                    audio.handle_set_volume(value);
                }
                Ok(AudioCommand::Seek(seconds)) => {
                    audio.handle_seek(seconds, Some(&stream_handle));
                }
                Ok(AudioCommand::SetMute(mute)) => {
                    audio.handle_set_mute(mute);
                }
                Ok(AudioCommand::GetStatus(tx)) => {
                    let status = audio.get_status();
                    let _ = tx.send(status);
                }
                Ok(AudioCommand::Shutdown) => {
                    break;
                }
                Ok(AudioCommand::SetSpeed(value)) => {
                    audio.handle_set_speed(value);
                }
                Ok(AudioCommand::SetEq(band, gain)) => {
                    // Values are already updated in shared mutex by the caller usually,
                    // but if we want to be safe or if caller didn't update mutex:
                    // In this design, caller updates mutex. Command is just for signaling.
                    // Optionally we could force update here if we passed value only.
                    // But wait, the Mutex is shared between generic AudioPlayer and AudioThread.
                    // The Equalizer Struct ALSO has the Arc.
                    println!("[AudioThread] EQ changed: band {} -> {} dB", band, gain);
                }
                Ok(AudioCommand::SetReverb(mix, decay)) => {
                    println!("[AudioThread] Reverb set: mix={}, decay={}", mix, decay);
                }
                Ok(AudioCommand::GetVisualizerData(tx)) => {
                    // This path is not typically used since we read directly from FftProcessor,
                    // but we keep it for potential future use
                    let _ = tx.send(VisualizerData::default());
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Check if track finished
                    if audio.state == PlayerState::Playing {
                        if let Some(ref sink) = audio.sink {
                            // Grace period: Only check for completion if we've been playing for at least 500ms
                            let elapsed = audio
                                .play_start_time
                                .map(|t| t.elapsed().as_millis())
                                .unwrap_or(0);

                            if elapsed > 500 && sink.empty() {
                                println!("[Audio] Track finished naturally");
                                audio.handle_stop();
                            }
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }
    }

    fn handle_play(&mut self, path: &str, stream_handle: &Arc<rodio::OutputStreamHandle>) {
        println!("[AudioThread] Handling play for path: '{}'", path);
        // Stop current playback
        self.handle_stop();

        let path = Path::new(path);

        // Open and decode the file
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[AudioThread] Failed to open file: {}", e);
                return;
            }
        };
        // Increase buffer size to prevent underruns (static/breaking)
        let reader = BufReader::with_capacity(128 * 1024, file);
        let source = match Decoder::new(reader) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to decode audio: {}", e);
                return;
            }
        };

        // Extract metadata
        let track_info = self.extract_metadata(path);

        // Create new sink and play
        let sink = match Sink::try_new(stream_handle) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to create audio sink: {}", e);
                return;
            }
        };

        sink.set_volume(self.volume);

        // Wrap source in processing chain:
        // Decoder -> f32 -> VisualizerTap (for FFT) -> Equalizer -> Sink
        let source_f32 = source.convert_samples::<f32>();
        let tapped = VisualizerTap::new(source_f32, Arc::clone(&self.fft_buffer));
        let equalizer = Equalizer::new(tapped, self.eq_gains.clone());
        sink.append(equalizer);

        self.sink = Some(sink);
        self.state = PlayerState::Playing;
        self.current_track = Some(track_info);
        self.current_path = Some(path.to_string_lossy().to_string());
        self.play_start_time = Some(Instant::now());
        self.accumulated_time = 0.0;
    }

    fn handle_load(&mut self, path: &str) {
        println!("[AudioThread] Handling load for path: '{}'", path);
        // Stop current playback
        self.handle_stop();

        let path_obj = Path::new(path);
        // Extract metadata
        let track_info = self.extract_metadata(path_obj);

        self.state = PlayerState::Paused; // Load starts in paused state
        self.current_track = Some(track_info);
        self.current_path = Some(path.to_string());
        self.play_start_time = None;
        self.accumulated_time = 0.0;
    }

    fn extract_metadata(&self, path: &Path) -> TrackInfo {
        let tagged_file = match Probe::open(path).and_then(|p| p.read()) {
            Ok(f) => f,
            Err(_) => {
                return TrackInfo {
                    path: path.to_string_lossy().to_string(),
                    title: path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown")
                        .to_string(),
                    artist: "Unknown Artist".to_string(),
                    album: "Unknown Album".to_string(),
                    duration_secs: 0.0,
                    cover_image: None,
                    disc_number: None,
                    track_number: None,
                };
            }
        };

        let properties = tagged_file.properties();
        let duration_secs = properties.duration().as_secs_f64();

        let (title, artist, album) = if let Some(tag) = tagged_file.primary_tag() {
            (
                tag.title().map(|s| s.to_string()).unwrap_or_else(|| {
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown")
                        .to_string()
                }),
                tag.artist()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Unknown Artist".to_string()),
                tag.album()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Unknown Album".to_string()),
            )
        } else {
            (
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string(),
                "Unknown Artist".to_string(),
                "Unknown Album".to_string(),
            )
        };

        TrackInfo {
            path: path.to_string_lossy().to_string(),
            title,
            artist,
            album,
            duration_secs,
            cover_image: None,
            disc_number: None,
            track_number: None,
        }
    }

    fn handle_pause(&mut self) {
        if let Some(ref sink) = self.sink {
            sink.pause();

            // Update accumulated time
            if let Some(start) = self.play_start_time {
                self.accumulated_time += start.elapsed().as_secs_f64();
            }
            self.play_start_time = None;
            self.state = PlayerState::Paused;
        }
    }

    fn handle_resume(&mut self) {
        if let Some(ref sink) = self.sink {
            sink.play();
            self.play_start_time = Some(Instant::now());
            self.state = PlayerState::Playing;
        }
    }

    fn handle_stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.state = PlayerState::Stopped;
        self.current_track = None;
        self.play_start_time = None;
        self.accumulated_time = 0.0;
    }

    fn handle_set_volume(&mut self, value: f32) {
        self.volume = value.clamp(0.0, 1.0);
        if !self.muted {
            if let Some(ref sink) = self.sink {
                sink.set_volume(self.volume);
            }
        }
    }

    fn handle_set_mute(&mut self, mute: bool) {
        self.muted = mute;
        if let Some(ref sink) = self.sink {
            if mute {
                sink.set_volume(0.0);
            } else {
                sink.set_volume(self.volume);
            }
        }
    }

    fn handle_set_speed(&mut self, value: f32) {
        if let Some(ref sink) = self.sink {
            sink.set_speed(value);
        }
    }

    fn handle_seek(
        &mut self,
        seconds: f64,
        stream_handle: Option<&Arc<rodio::OutputStreamHandle>>,
    ) {
        println!("[Audio] Seeking to {} seconds", seconds);

        // First try native seek
        if let Some(ref mut sink) = self.sink {
            match sink.try_seek(std::time::Duration::from_secs_f64(seconds)) {
                Ok(_) => {
                    println!("[Audio] Native seek successful");
                    self.accumulated_time = seconds;
                    if self.state == PlayerState::Playing {
                        self.play_start_time = Some(Instant::now());
                    }
                    return;
                }
                Err(e) => {
                    println!("[Audio] Native seek failed: {:?}, trying reload method", e);
                }
            }
        }

        // Fallback: reload file and skip to position
        if let (Some(path), Some(stream_handle)) = (&self.current_path, stream_handle) {
            let was_playing = self.state == PlayerState::Playing;
            let track_info = self.current_track.clone();
            let path = path.clone();

            // Stop current playback
            if let Some(sink) = self.sink.take() {
                sink.stop();
            }

            // Reload and skip
            let file = match File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("[Audio] Seek reload failed: {}", e);
                    return;
                }
            };

            let reader = BufReader::with_capacity(128 * 1024, file);
            let source = match Decoder::new(reader) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[Audio] Seek decode failed: {}", e);
                    return;
                }
            };

            // Skip to the target position using skip_duration
            let skipped_source = source.skip_duration(std::time::Duration::from_secs_f64(seconds));

            let sink = match Sink::try_new(stream_handle) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[Audio] Seek sink creation failed: {}", e);
                    return;
                }
            };

            sink.set_volume(self.volume);

            // Wrap source in processing chain (same as handle_play)
            let source_f32 = skipped_source.convert_samples::<f32>();
            let tapped = VisualizerTap::new(source_f32, Arc::clone(&self.fft_buffer));
            let equalizer = Equalizer::new(tapped, self.eq_gains.clone());
            sink.append(equalizer);

            if !was_playing {
                sink.pause();
            }

            self.sink = Some(sink);
            self.current_track = track_info;
            self.current_path = Some(path);
            self.accumulated_time = seconds;
            self.state = if was_playing {
                PlayerState::Playing
            } else {
                PlayerState::Paused
            };
            self.play_start_time = if was_playing {
                Some(Instant::now())
            } else {
                None
            };

            println!("[Audio] Seek via reload successful");
        } else {
            println!("[Audio] Seek failed: no path or stream handle");
        }
    }

    fn get_status(&self) -> PlayerStatus {
        let mut position_secs = {
            let current = self
                .play_start_time
                .map(|start| start.elapsed().as_secs_f64())
                .unwrap_or(0.0);
            self.accumulated_time + current
        };

        // Cap position to duration to prevent exceeding
        if let Some(ref track) = self.current_track {
            if position_secs > track.duration_secs {
                position_secs = track.duration_secs;
            }
        }

        PlayerStatus {
            state: self.state,
            track: self.current_track.clone(),
            position_secs,
            volume: self.volume,
        }
    }
}
