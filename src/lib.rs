pub mod platform;
pub(crate) mod protocol;
pub mod utils;
pub mod webview;
pub mod window;
use crate::utils::error::Error;
pub use crate::protocol::{ipc::{IpcBody,IpcRequest,IpcResponse,IpcResponseBody}};
/// Result type.
pub type Result<T> = std::result::Result<T, Error>;
