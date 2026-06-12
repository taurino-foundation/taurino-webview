use std::borrow::Cow;

use http::{
    Response, StatusCode,
    header::{
        ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
        CONTENT_TYPE,
    },
};
#[allow(dead_code)]
pub(crate) const TAURINO_CALLBACK_HEADER_NAME: &str = "Taurino-Callback";
#[allow(dead_code)]
pub(crate) const TAURINO_ERROR_HEADER_NAME: &str = "Taurino-Error";
pub(crate) const TAURINO_INVOKE_KEY_HEADER_NAME: &str = "Taurino-Invoke-Key";
pub(crate) const TAURINO_RESPONSE_HEADER_NAME: &str = "Taurino-Response";

pub(crate) const TAURINO_RESPONSE_OK: &str = "ok";
pub(crate) const TAURINO_RESPONSE_ERROR: &str = "error";

pub(crate) const ALLOWED_IPC_HEADERS: &str =
    "content-type, taurino-callback, taurino-error, taurino-invoke-key";

pub(crate) const EXPOSED_IPC_HEADERS: &str = "Taurino-Response";

/// Raw IPC body received from JavaScript.
///
/// The manager does not deserialize this into command arguments.
/// That is the user's responsibility.
#[derive(Debug, Clone)]
pub enum IpcBody {
    Json(serde_json::Value),
    Raw(Vec<u8>),
}

/// Raw IPC request passed to the user-defined IPC system.
///
/// This type describes how the data arrives from JavaScript.
/// It intentionally does not know anything about application commands.
#[derive(Debug, Clone)]
pub struct IpcRequest {
    /// Window label that owns the webview.
    pub window_label: String,

    /// Internal webview id.
    pub webview_id: String,

    /// Command name from:
    ///
    /// JS:
    /// `invoke("my_command", payload)`
    ///
    /// Rust:
    /// `command == "my_command"`
    pub command: String,

    /// IPC body sent by JavaScript.
    ///
    /// JSON payloads become `IpcBody::Json`.
    /// Binary payloads become `IpcBody::Raw`.
    pub body: IpcBody,

    /// Original request headers.
    pub headers: http::HeaderMap,

    /// Content-Type used by JavaScript.
    pub content_type: String,

    /// Optional Origin header.
    pub origin: Option<String>,

    /// Runtime invoke key sent by JavaScript.
    pub invoke_key: Option<String>,
}

/// Body returned from the user's IPC system.
#[derive(Debug, Clone)]
pub enum IpcResponseBody {
    Json(serde_json::Value),
    Text(String),
    Raw(Vec<u8>),
}

/// IPC response returned by the user-defined IPC system.
///
/// `ok == true` resolves the JavaScript Promise.
/// `ok == false` rejects the JavaScript Promise.
#[derive(Debug, Clone)]
pub struct IpcResponse {
    pub ok: bool,
    pub body: IpcResponseBody,
}

impl IpcResponse {
    /// Resolve the JS Promise with JSON.
    pub fn resolve_json(value: serde_json::Value) -> Self {
        Self {
            ok: true,
            body: IpcResponseBody::Json(value),
        }
    }

    /// Reject the JS Promise with JSON.
    pub fn reject_json(value: serde_json::Value) -> Self {
        Self {
            ok: false,
            body: IpcResponseBody::Json(value),
        }
    }

    /// Resolve the JS Promise with plain text.
    pub fn resolve_text(value: impl Into<String>) -> Self {
        Self {
            ok: true,
            body: IpcResponseBody::Text(value.into()),
        }
    }

    /// Reject the JS Promise with plain text.
    pub fn reject_text(value: impl Into<String>) -> Self {
        Self {
            ok: false,
            body: IpcResponseBody::Text(value.into()),
        }
    }

    /// Resolve the JS Promise with raw bytes.
    pub fn resolve_raw(value: Vec<u8>) -> Self {
        Self {
            ok: true,
            body: IpcResponseBody::Raw(value),
        }
    }

    /// Reject the JS Promise with raw bytes.
    pub fn reject_raw(value: Vec<u8>) -> Self {
        Self {
            ok: false,
            body: IpcResponseBody::Raw(value),
        }
    }
}

pub(crate) fn parse_ipc_body(content_type: &str, body: &[u8]) -> IpcBody {
    let content_type = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    if content_type == "application/json" {
        match serde_json::from_slice::<serde_json::Value>(body) {
            Ok(value) => IpcBody::Json(value),
            Err(_) => IpcBody::Raw(body.to_vec()),
        }
    } else {
        IpcBody::Raw(body.to_vec())
    }
}

pub(crate) fn build_ipc_http_response(
    allowed_origin: &str,
    response: IpcResponse,
) -> Response<Cow<'static, [u8]>> {
    let taurino_response = if response.ok {
        TAURINO_RESPONSE_OK
    } else {
        TAURINO_RESPONSE_ERROR
    };

    let (content_type, body) = match response.body {
        IpcResponseBody::Json(value) => ("application/json", value.to_string().into_bytes()),
        IpcResponseBody::Text(value) => ("text/plain; charset=utf-8", value.into_bytes()),
        IpcResponseBody::Raw(value) => ("application/octet-stream", value),
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .header(TAURINO_RESPONSE_HEADER_NAME, taurino_response)
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, allowed_origin)
        .header(ACCESS_CONTROL_ALLOW_METHODS, "POST, OPTIONS")
        .header(ACCESS_CONTROL_ALLOW_HEADERS, ALLOWED_IPC_HEADERS)
        .header("Access-Control-Expose-Headers", EXPOSED_IPC_HEADERS)
        .body(Cow::Owned(body))
        .expect("failed to build IPC response")
}
