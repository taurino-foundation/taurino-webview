pub mod platform;
pub(crate) mod protocol;
pub mod utils;
pub mod webview;
pub mod window;
use crate::utils::error::Error;

/// Result type.
pub type Result<T> = std::result::Result<T, Error>;
