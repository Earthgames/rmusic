pub mod audio_conversion;
pub mod database;
pub mod decoders;
pub mod playback;
pub mod queue;

/// Shorthand for Result
pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
