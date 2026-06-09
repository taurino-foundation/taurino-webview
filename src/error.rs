use std::path::PathBuf;

use http::{
    header::{InvalidHeaderName, InvalidHeaderValue},
    method::InvalidMethod,
    status::InvalidStatusCode,
};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(
        "invalid extension `{extension}` used for image {path}, must be `ico` or `png`"
    )]
    InvalidImageExtension { extension: PathBuf, path: PathBuf },
    /// Failed to create webview.
    #[error("failed to create webview: {0}")]
    CreateWebview(Box<dyn std::error::Error + Send + Sync>),
    // TODO: Make it take an error like `CreateWebview` in v3
    /// Failed to serialize/deserialize.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// Failed to get cursor position.
    #[error("failed to get cursor position")]
    FailedToGetCursorPosition,
    #[error("Invalid header name: {0}")]
    InvalidHeaderName(#[from] InvalidHeaderName),
    #[error("Invalid header value: {0}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
    #[error("Invalid status code: {0}")]
    InvalidStatusCode(#[from] InvalidStatusCode),
    #[error("Invalid method: {0}")]
    InvalidMethod(#[from] InvalidMethod),
    #[error("Infallible error, something went really wrong: {0}")]
    Infallible(#[from] std::convert::Infallible),
    #[error("Invalid proxy url")]
    InvalidProxyUrl,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    #[error("failed to remove data store")]
    FailedToRemoveDataStore,
    #[error("Could not find the webview runtime, make sure it is installed")]
    WebviewRuntimeNotInstalled,
    /// Window label must be unique.
    #[error("a window with label `{0}` already exists")]
    WindowLabelAlreadyExists(String),
    /// Webview label must be unique.
    #[error("a webview with label `{0}` already exists")]
    WebviewLabelAlreadyExists(String),
    /// IO error.
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// Failed to load window icon.
    #[error("invalid icon: {0}")]
    InvalidIcon(std::io::Error),
    /// A part of the URL is malformed or invalid. This may occur when parsing and combining
    /// user-provided URLs and paths.
    #[error("invalid url: {0}")]
    InvalidUrl(url::ParseError),
    #[error("async task join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
}
