pub mod async_runtime;
pub mod attributes;
pub mod builder;
pub mod error;
pub mod events;
pub mod factory;
pub mod manager;
/* pub mod pattern; */
pub mod pending;
pub mod platform;
pub(crate) mod protocol;
pub mod types;
pub mod utils;
pub mod webview;
pub mod wrapper;

use crate::error::Error;

/// Result type.
pub type Result<T> = std::result::Result<T, Error>;
