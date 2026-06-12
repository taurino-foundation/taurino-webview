use std::sync::Arc;

use http::{
    HeaderValue, Request, Response, StatusCode,
    header::{CONTENT_SECURITY_POLICY, CONTENT_TYPE},
};

use crate::{protocol::pattern::RuntimeAssets, utils::ManagerUriSchemeResponder};

/// Protocol handler for serving runtime isolation assets.
///
/// Unlike Tauri's `EmbeddedAssets` protocol, this handler serves assets that
/// are provided at runtime. This makes it suitable for Python integrations
/// where HTML, JavaScript, CSS, and other files are passed as `bytes`.
pub struct RuntimeIsolationProtocol {
    /// The runtime asset collection used by the isolation frame.
    assets: Arc<RuntimeAssets>,

    /// The origin of the main application window.
    ///
    /// This can be injected into the isolation runtime so the secure frame
    /// knows which origin it is allowed to communicate with.
    window_origin: String,

    /// The CSP frame source for the isolation frame.
    frame_src: String,

    /// Whether the runtime uses an HTTPS-compatible custom scheme.
    use_https_scheme: bool,
}

/// Creates a runtime isolation protocol handler.
///
/// The returned handler serves files from `RuntimeAssets`.
///
/// `schema` is the custom isolation scheme, for example:
///
/// - `isolation-abc123`
///
/// On Linux/macOS the frame source becomes:
///
/// - `isolation-abc123:`
///
/// On Windows/Android the frame source becomes:
///
/// - `http://isolation-abc123.localhost`
/// - `https://isolation-abc123.localhost`
pub fn get(
    schema: String,
    assets: Arc<RuntimeAssets>,
    window_origin: String,
    use_https_scheme: bool,
) -> Arc<RuntimeIsolationProtocol> {
    let frame_src = if cfg!(windows) || cfg!(target_os = "android") {
        let scheme = if use_https_scheme { "https" } else { "http" };
        format!("{scheme}://{schema}.localhost")
    } else {
        format!("{schema}:")
    };

    Arc::new(RuntimeIsolationProtocol {
        assets,
        window_origin,
        frame_src,
        use_https_scheme,
    })
}

impl RuntimeIsolationProtocol {
    /// Handles one isolation protocol request.
    ///
    /// Only assets from `RuntimeAssets` are served. Unknown paths return `404`.
    pub fn handle(
        &self,
        _webview_id: &str,
        request: Request<Vec<u8>>,
        responder: ManagerUriSchemeResponder,
    ) {
        let path = request_to_asset_path(&request);

        let response = match self.assets.get(&path) {
            Some(asset) => self.asset_response(&path, asset.as_bytes(), asset.mime()),
            None => not_found_response(),
        };

        responder.respond(response);
    }

    /// Builds a response for a runtime asset.
    ///
    /// `index.html` receives the isolation runtime bootstrap.
    /// Other assets are served as-is.
    fn asset_response(&self, path: &str, bytes: &[u8], mime: &str) -> Response<Vec<u8>> {
        if path == "index.html" {
            self.index_response(bytes)
        } else {
            response_with_body(StatusCode::OK, mime, bytes.to_vec())
        }
    }

    /// Builds the isolation `index.html` response.
    ///
    /// This is the place where runtime JavaScript is injected.
    /// Keep this small and explicit so it stays independent from Tauri internals.
    fn index_response(&self, bytes: &[u8]) -> Response<Vec<u8>> {
        let html = String::from_utf8_lossy(bytes);

        let runtime_script = self.runtime_script();

        let html = inject_runtime_script(&html, &runtime_script);

        let mut response = response_with_body(
            StatusCode::OK,
            "text/html; charset=utf-8",
            html.into_bytes(),
        );

        let csp = self.content_security_policy();

        if let Ok(value) = HeaderValue::from_str(&csp) {
            response
                .headers_mut()
                .insert(CONTENT_SECURITY_POLICY, value);
        }

        response
    }

    /// Generates the JavaScript runtime injected into the isolation page.
    ///
    /// This replaces Tauri's `IsolationJavascriptRuntime`.
    /// You can extend this later with encryption, IPC validation, or message keys.
    fn runtime_script(&self) -> String {
        format!(
            r#"
<script>
(() => {{
    "use strict";

    const WINDOW_ORIGIN = {window_origin};
    const USE_HTTPS_SCHEME = {use_https_scheme};

    Object.defineProperty(window, "__TAURINO_ISOLATION__", {{
        value: Object.freeze({{
            origin: WINDOW_ORIGIN,
            useHttpsScheme: USE_HTTPS_SCHEME
        }}),
        configurable: false,
        enumerable: false,
        writable: false
    }});

    window.addEventListener("message", (event) => {{
        if (event.origin !== WINDOW_ORIGIN) {{
            return;
        }}

        if (!event.data || typeof event.data !== "object") {{
            return;
        }}

        if (window.parent) {{
            window.parent.postMessage(event.data, WINDOW_ORIGIN);
        }}
    }});
}})();
</script>
"#,
            window_origin = serde_json::to_string(&self.window_origin)
                .unwrap_or_else(|_| "\"null\"".to_string()),
            use_https_scheme = self.use_https_scheme,
        )
    }

    /// Builds a strict default CSP for the isolation document.
    ///
    /// You can loosen this later if your isolation assets need fonts, images,
    /// styles, or additional script sources.
    fn content_security_policy(&self) -> String {
        format!(
            "default-src 'none'; \
             frame-src {frame_src}; \
             script-src 'unsafe-inline'; \
             style-src 'unsafe-inline'; \
             img-src data: blob:; \
             connect-src 'none'; \
             base-uri 'none'; \
             form-action 'none'",
            frame_src = self.frame_src,
        )
    }
}

/// Converts an HTTP request URI into a normalized asset path.
///
/// Examples:
///
/// - `/` -> `index.html`
/// - `/index.html` -> `index.html`
/// - `/main.js` -> `main.js`
/// - `/assets/app.css` -> `assets/app.css`
fn request_to_asset_path(request: &Request<Vec<u8>>) -> String {
    let raw_path = request.uri().path();

    let decoded = percent_encoding::percent_decode(raw_path.as_bytes())
        .decode_utf8_lossy()
        .to_string();

    normalize_asset_path(&decoded)
}

/// Normalizes a request path to the key format used by `RuntimeAssets`.
fn normalize_asset_path(path: &str) -> String {
    let path = path.trim();

    if path.is_empty() || path == "/" {
        return "index.html".to_string();
    }

    let path = path
        .trim_start_matches('/')
        .trim_end_matches('/')
        .replace('\\', "/");

    if path.is_empty() {
        "index.html".to_string()
    } else {
        path
    }
}

/// Injects the runtime script into an HTML document.
///
/// The script is inserted before `</head>` if possible. If no `</head>` exists,
/// it is prepended to the document.
fn inject_runtime_script(html: &str, runtime_script: &str) -> String {
    if let Some(index) = html.find("</head>") {
        let mut output = String::with_capacity(html.len() + runtime_script.len());
        output.push_str(&html[..index]);
        output.push_str(runtime_script);
        output.push_str(&html[index..]);
        output
    } else if let Some(index) = html.find("</body>") {
        let mut output = String::with_capacity(html.len() + runtime_script.len());
        output.push_str(&html[..index]);
        output.push_str(runtime_script);
        output.push_str(&html[index..]);
        output
    } else {
        format!("{runtime_script}{html}")
    }
}

/// Creates a simple HTTP response with body and content type.
fn response_with_body(status: StatusCode, content_type: &str, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, content_type)
        .body(body)
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(CONTENT_TYPE, "text/plain; charset=utf-8")
                .body(b"failed to build response".to_vec())
                .expect("failed to build fallback response")
        })
}

/// Creates a `404 Not Found` response.
fn not_found_response() -> Response<Vec<u8>> {
    response_with_body(
        StatusCode::NOT_FOUND,
        "text/plain; charset=utf-8",
        b"not found".to_vec(),
    )
}
