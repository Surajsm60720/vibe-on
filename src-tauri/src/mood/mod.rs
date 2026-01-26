// Mood-based playback feature module
// This entire module can be removed for clean feature removal

pub mod analyzer;
pub mod commands;
pub mod db;
pub mod schema;
pub mod types;

pub use commands::*;
