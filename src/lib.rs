pub mod playback;
pub mod decoders;

/// Shorthand for Result
pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
