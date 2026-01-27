// Mood-based playback feature module
// This entire module can be removed for clean feature removal

pub mod analyzer;
pub mod audio_loader;
pub mod commands;
pub mod db;
#[cfg(debug_assertions)]
pub mod debug;
pub mod dsp;
pub mod rust_analyzer;
pub mod schema;
pub mod types;

pub use commands::*;
