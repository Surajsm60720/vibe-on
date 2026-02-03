//! Adaptive audio buffer for P2P streaming
//!
//! Two modes:
//! - Full pre-buffer: For files ≤20MB, entire file is buffered before playback
//! - Ring buffer: For larger files, 3-second rolling buffer with streaming

use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rodio::Source;

/// Threshold for pre-buffering entire file (20MB)
pub const PREBUFFER_THRESHOLD: u64 = 20 * 1024 * 1024;

/// Size of ring buffer in seconds
pub const RING_BUFFER_SECONDS: f32 = 3.0;

/// Buffer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferState {
    /// Waiting for data
    Buffering,
    /// Ready to play
    Ready,
    /// Stream ended
    Finished,
    /// Error occurred
    Error,
}

/// Buffer mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferMode {
    /// Pre-buffer entire file
    PreBuffer,
    /// Stream with ring buffer
    RingBuffer,
}

/// Shared buffer data
pub struct BufferData {
    /// Audio data bytes
    data: Vec<u8>,
    /// Current read position
    read_pos: usize,
    /// Total expected size (for pre-buffer mode)
    total_size: Option<u64>,
    /// Whether all data has been received
    complete: bool,
    /// Current state
    state: BufferState,
    /// Buffer mode
    mode: BufferMode,
    /// Audio format info
    format: Option<AudioFormat>,
}

/// Audio format information
#[derive(Debug, Clone)]
pub struct AudioFormat {
    pub format_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_secs: f64,
}

impl BufferData {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            read_pos: 0,
            total_size: None,
            complete: false,
            state: BufferState::Buffering,
            mode: BufferMode::PreBuffer,
            format: None,
        }
    }
    
    /// Set the expected total size and determine buffer mode
    pub fn set_total_size(&mut self, size: u64) {
        self.total_size = Some(size);
        self.mode = if size <= PREBUFFER_THRESHOLD {
            BufferMode::PreBuffer
        } else {
            BufferMode::RingBuffer
        };
        
        // Pre-allocate for pre-buffer mode
        if self.mode == BufferMode::PreBuffer {
            self.data.reserve(size as usize);
        }
    }
    
    /// Set audio format information
    pub fn set_format(&mut self, format: AudioFormat) {
        self.format = Some(format);
    }
    
    /// Append data to the buffer
    pub fn append(&mut self, chunk: &[u8]) {
        self.data.extend_from_slice(chunk);
        
        // Check if pre-buffer is complete
        if self.mode == BufferMode::PreBuffer {
            if let Some(total) = self.total_size {
                if self.data.len() >= total as usize {
                    self.complete = true;
                    self.state = BufferState::Ready;
                }
            }
        } else {
            // For ring buffer, we're ready once we have enough data
            if let Some(ref format) = self.format {
                let bytes_per_second = (format.sample_rate * format.channels as u32 * 2) as usize;
                let required = (bytes_per_second as f32 * RING_BUFFER_SECONDS) as usize;
                
                if self.data.len() >= required && self.state == BufferState::Buffering {
                    self.state = BufferState::Ready;
                }
            }
        }
    }
    
    /// Mark the stream as complete
    pub fn mark_complete(&mut self) {
        self.complete = true;
        if self.state == BufferState::Buffering {
            self.state = BufferState::Ready;
        }
    }
    
    /// Read bytes from the buffer
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let available = self.data.len().saturating_sub(self.read_pos);
        let to_read = buf.len().min(available);
        
        if to_read > 0 {
            buf[..to_read].copy_from_slice(&self.data[self.read_pos..self.read_pos + to_read]);
            self.read_pos += to_read;
        }
        
        // Check if we've read everything and stream is complete
        if self.complete && self.read_pos >= self.data.len() {
            self.state = BufferState::Finished;
        }
        
        to_read
    }
    
    /// Seek to a position
    pub fn seek(&mut self, pos: u64) -> bool {
        if self.mode == BufferMode::PreBuffer && pos < self.data.len() as u64 {
            self.read_pos = pos as usize;
            true
        } else {
            // Ring buffer mode requires re-streaming
            false
        }
    }
    
    /// Get current state
    pub fn state(&self) -> BufferState {
        self.state
    }
    
    /// Get buffer mode
    pub fn mode(&self) -> BufferMode {
        self.mode
    }
    
    /// Get buffered percentage
    pub fn buffered_percent(&self) -> f32 {
        match self.total_size {
            Some(total) if total > 0 => (self.data.len() as f32 / total as f32) * 100.0,
            _ => 0.0,
        }
    }
    
    /// Clear the buffer
    pub fn clear(&mut self) {
        self.data.clear();
        self.read_pos = 0;
        self.total_size = None;
        self.complete = false;
        self.state = BufferState::Buffering;
        self.format = None;
    }
    
    /// Check if buffer can seek (pre-buffer mode with complete data)
    pub fn can_seek(&self) -> bool {
        self.mode == BufferMode::PreBuffer && self.complete
    }
    
    /// Get the raw data for decoding
    pub fn get_data(&self) -> &[u8] {
        &self.data
    }
    
    /// Get current read position
    pub fn read_position(&self) -> usize {
        self.read_pos
    }
}

/// Thread-safe adaptive buffer
#[derive(Clone)]
pub struct AdaptiveBuffer {
    inner: Arc<Mutex<BufferData>>,
}

impl AdaptiveBuffer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(BufferData::new())),
        }
    }
    
    /// Set expected total size
    pub fn set_total_size(&self, size: u64) {
        if let Ok(mut data) = self.inner.lock() {
            data.set_total_size(size);
        }
    }
    
    /// Set audio format
    pub fn set_format(&self, format: AudioFormat) {
        if let Ok(mut data) = self.inner.lock() {
            data.set_format(format);
        }
    }
    
    /// Append chunk to buffer
    pub fn append(&self, chunk: &[u8]) {
        if let Ok(mut data) = self.inner.lock() {
            data.append(chunk);
        }
    }
    
    /// Mark stream as complete
    pub fn mark_complete(&self) {
        if let Ok(mut data) = self.inner.lock() {
            data.mark_complete();
        }
    }
    
    /// Get current state
    pub fn state(&self) -> BufferState {
        self.inner.lock().map(|d| d.state()).unwrap_or(BufferState::Error)
    }
    
    /// Get buffer mode
    pub fn mode(&self) -> BufferMode {
        self.inner.lock().map(|d| d.mode()).unwrap_or(BufferMode::PreBuffer)
    }
    
    /// Get buffered percentage
    pub fn buffered_percent(&self) -> f32 {
        self.inner.lock().map(|d| d.buffered_percent()).unwrap_or(0.0)
    }
    
    /// Check if ready to play
    pub fn is_ready(&self) -> bool {
        self.state() == BufferState::Ready
    }
    
    /// Can seek without re-streaming
    pub fn can_seek(&self) -> bool {
        self.inner.lock().map(|d| d.can_seek()).unwrap_or(false)
    }
    
    /// Seek to byte position (returns false if re-stream needed)
    pub fn seek(&self, pos: u64) -> bool {
        self.inner.lock().map(|mut d| d.seek(pos)).unwrap_or(false)
    }
    
    /// Clear the buffer
    pub fn clear(&self) {
        if let Ok(mut data) = self.inner.lock() {
            data.clear();
        }
    }
    
    /// Read bytes
    pub fn read(&self, buf: &mut [u8]) -> usize {
        self.inner.lock().map(|mut d| d.read(buf)).unwrap_or(0)
    }
    
    /// Get a clone of the data for decoding
    pub fn get_data_clone(&self) -> Option<Vec<u8>> {
        self.inner.lock().ok().map(|d| d.get_data().to_vec())
    }
    
    /// Get audio format
    pub fn get_format(&self) -> Option<AudioFormat> {
        self.inner.lock().ok().and_then(|d| d.format.clone())
    }
}

impl Default for AdaptiveBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// A rodio-compatible source that reads from an adaptive buffer
/// This wraps an inner decoder created from the buffered data
pub struct StreamingSource {
    buffer: AdaptiveBuffer,
    decoder: Option<rodio::Decoder<Cursor<Vec<u8>>>>,
    sample_rate: u32,
    channels: u16,
}

impl StreamingSource {
    /// Create a new streaming source from a buffer
    /// 
    /// Note: The buffer should be in Ready state with complete pre-buffered data
    pub fn new(buffer: AdaptiveBuffer) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let format = buffer.get_format();
        let sample_rate = format.as_ref().map(|f| f.sample_rate).unwrap_or(44100);
        let channels = format.as_ref().map(|f| f.channels).unwrap_or(2);
        
        // For pre-buffered files, create decoder from the full data
        let decoder = if buffer.is_ready() {
            if let Some(data) = buffer.get_data_clone() {
                let cursor = Cursor::new(data);
                Some(rodio::Decoder::new(cursor)?)
            } else {
                None
            }
        } else {
            None
        };
        
        Ok(Self {
            buffer,
            decoder,
            sample_rate,
            channels,
        })
    }
    
    /// Try to initialize the decoder if not already done
    fn try_init_decoder(&mut self) {
        if self.decoder.is_none() && self.buffer.is_ready() {
            if let Some(data) = self.buffer.get_data_clone() {
                let cursor = Cursor::new(data);
                if let Ok(decoder) = rodio::Decoder::new(cursor) {
                    self.decoder = Some(decoder);
                }
            }
        }
    }
}

impl Iterator for StreamingSource {
    type Item = i16;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.try_init_decoder();
        self.decoder.as_mut().and_then(|d| d.next())
    }
}

impl Source for StreamingSource {
    fn current_frame_len(&self) -> Option<usize> {
        self.decoder.as_ref().and_then(|d| d.current_frame_len())
    }
    
    fn channels(&self) -> u16 {
        self.decoder.as_ref().map(|d| d.channels()).unwrap_or(self.channels)
    }
    
    fn sample_rate(&self) -> u32 {
        self.decoder.as_ref().map(|d| d.sample_rate()).unwrap_or(self.sample_rate)
    }
    
    fn total_duration(&self) -> Option<Duration> {
        self.decoder.as_ref().and_then(|d| d.total_duration())
    }
}
