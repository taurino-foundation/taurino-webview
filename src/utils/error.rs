use std::path::PathBuf;

use http::{
    header::{InvalidHeaderName, InvalidHeaderValue},
    method::InvalidMethod,
    status::InvalidStatusCode,
};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The image file extension is not supported.
    #[error(
        "unsupported image extension `{extension}` for image `{path}`; expected `ico` or `png`"
    )]
    InvalidImageExtension { extension: PathBuf, path: PathBuf },

    /// Failed to create the webview.
    #[error("failed to create webview: {0}")]
    CreateWebview(Box<dyn std::error::Error + Send + Sync>),

    /// Failed to create the window.
    #[error("failed to create window: {0}")]
    CreateWindow(tao::error::OsError),

    /// Failed to serialize or deserialize JSON data.
    #[error("failed to process JSON data: {0}")]
    Json(#[from] serde_json::Error),

    /// Failed to get the current cursor position.
    #[error("failed to retrieve the current cursor position")]
    FailedToGetCursorPosition,

    /// Invalid HTTP header name.
    #[error("invalid HTTP header name: {0}")]
    InvalidHeaderName(#[from] InvalidHeaderName),

    /// Invalid HTTP header value.
    #[error("invalid HTTP header value: {0}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),

    /// Invalid HTTP status code.
    #[error("invalid HTTP status code: {0}")]
    InvalidStatusCode(#[from] InvalidStatusCode),

    /// Invalid HTTP method.
    #[error("invalid HTTP method: {0}")]
    InvalidMethod(#[from] InvalidMethod),

    /// An infallible operation unexpectedly failed.
    #[error("an unexpected infallible error occurred: {0}")]
    Infallible(#[from] std::convert::Infallible),

    /// Invalid proxy URL.
    #[error("invalid proxy URL")]
    InvalidProxyUrl,

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    /// Failed to remove the webview data store.
    #[error("failed to remove the webview data store")]
    FailedToRemoveDataStore,

    /// The required webview runtime is not installed.
    #[error(
        "webview runtime not found; please make sure the required runtime is installed"
    )]
    WebviewRuntimeNotInstalled,

    /// Window label must be unique.
    #[error("a window with the label `{0}` already exists")]
    WindowLabelAlreadyExists(String),

    /// Webview label must be unique.
    #[error("a webview with the label `{0}` already exists")]
    WebviewLabelAlreadyExists(String),

    /// IO error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to load or validate a window icon from an IO source.
    #[error("failed to load icon: {0}")]
    InvalidIcon(std::io::Error),

    /// Failed to validate a Tao window icon.
    #[error("invalid window icon: {0}")]
    InvalidTaoIcon(Box<tao::window::BadIcon>),

    /// A URL is malformed or invalid.
    #[error("invalid URL: {0}")]
    InvalidUrl(url::ParseError),

    /// An asynchronous task failed to join.
    #[error("async task failed to join: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    /// The requested operation is not supported.
    #[error("operation is not supported: {0}")]
    NotSupportedError(tao::error::NotSupportedError),

    /// An external error occurred outside of Tao's control.
    #[error("external platform error: {0}")]
    ExternalError(tao::error::ExternalError),
}
