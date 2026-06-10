use std::{collections::HashMap, sync::Arc};

use serde::Serialize;
use serialize_to_javascript::{Template, default_template};

/// The domain of the isolation iframe source.
pub const ISOLATION_IFRAME_SRC_DOMAIN: &str = "localhost";

/// Default isolation HTML path.
pub const DEFAULT_ISOLATION_INDEX: &str = "index.html";

/// Runtime asset used by the isolation pattern.
///
/// This is intentionally runtime-based and Python-friendly.
/// Python can pass `bytes`, Rust stores them as `Arc<[u8]>`.
#[derive(Debug, Clone)]
pub struct RuntimeAsset {
    pub bytes: Arc<[u8]>,
    pub mime: String,
}

impl RuntimeAsset {
    pub fn new<B, M>(bytes: B, mime: M) -> Self
    where
        B: Into<Vec<u8>>,
        M: Into<String>,
    {
        Self {
            bytes: Arc::<[u8]>::from(bytes.into()),
            mime: mime.into(),
        }
    }

    pub fn from_bytes<B>(path: &str, bytes: B) -> Self
    where
        B: Into<Vec<u8>>,
    {
        Self::new(bytes, mime_for_path(path))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn mime(&self) -> &str {
        &self.mime
    }
}

/// Runtime asset collection.
///
/// This replaces Tauri's compile-time `EmbeddedAssets` for your Python use case.
#[derive(Debug, Clone, Default)]
pub struct RuntimeAssets {
    assets: HashMap<String, RuntimeAsset>,
}

impl RuntimeAssets {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    /// Create isolation assets from a single HTML document.
    ///
    /// Python side:
    /// `ManagerConfig.isolation_html(b"<html>...</html>")`
    pub fn from_html<B>(html: B) -> Self
    where
        B: Into<Vec<u8>>,
    {
        let mut assets = Self::new();

        assets.insert(
            DEFAULT_ISOLATION_INDEX,
            RuntimeAsset::new(html, "text/html; charset=utf-8"),
        );

        assets
    }

    /// Create isolation assets from a map.
    ///
    /// Python side can later pass:
    ///
    /// {
    ///     "index.html": b"...",
    ///     "main.js": b"...",
    ///     "style.css": b"...",
    /// }
    pub fn from_bytes_map(map: HashMap<String, Vec<u8>>) -> Self {
        let mut assets = Self::new();

        for (path, bytes) in map {
            let normalized = normalize_asset_path(&path);
            let asset = RuntimeAsset::from_bytes(&normalized, bytes);
            assets.insert(normalized, asset);
        }

        assets
    }

    pub fn insert<P>(&mut self, path: P, asset: RuntimeAsset)
    where
        P: AsRef<str>,
    {
        let path = normalize_asset_path(path.as_ref());
        self.assets.insert(path, asset);
    }

    pub fn insert_bytes<P, B>(&mut self, path: P, bytes: B)
    where
        P: AsRef<str>,
        B: Into<Vec<u8>>,
    {
        let path = normalize_asset_path(path.as_ref());
        let asset = RuntimeAsset::from_bytes(&path, bytes);
        self.assets.insert(path, asset);
    }

    pub fn get<P>(&self, path: P) -> Option<&RuntimeAsset>
    where
        P: AsRef<str>,
    {
        let path = normalize_asset_path(path.as_ref());
        self.assets.get(&path)
    }

    pub fn get_index(&self) -> Option<&RuntimeAsset> {
        self.assets.get(DEFAULT_ISOLATION_INDEX)
    }

    pub fn contains<P>(&self, path: P) -> bool
    where
        P: AsRef<str>,
    {
        let path = normalize_asset_path(path.as_ref());
        self.assets.contains_key(&path)
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &RuntimeAsset)> {
        self.assets.iter()
    }
}

/// Runtime isolation keys.
///
/// For now this is intentionally simple.
/// Later you can replace this with real crypto keys or Tauri-compatible keys.
#[derive(Debug, Clone)]
pub struct RuntimeIsolationKeys {
    pub message_key: Arc<str>,
}

impl RuntimeIsolationKeys {
    pub fn new<K>(message_key: K) -> Self
    where
        K: Into<Arc<str>>,
    {
        Self {
            message_key: message_key.into(),
        }
    }

    pub fn generate() -> Self {
        Self {
            message_key: Arc::from(uuid::Uuid::new_v4().to_string()),
        }
    }
}

/// Application pattern.
#[derive(Debug, Clone)]
pub enum Pattern {
    /// Normal application pattern.
    Brownfield,

    /// Isolation pattern.
    ///
    /// This variant can be created from Python bytes at runtime.
    Isolation {
        /// Runtime assets served on the isolation protocol.
        assets: Arc<RuntimeAssets>,

        /// The schema used for the isolation frame.
        schema: String,

        /// Random message key.
        ///
        /// Should be regenerated per runtime.
        key: String,

        /// Runtime crypto/key data.
        crypto_keys: Arc<RuntimeIsolationKeys>,
    },
}

impl Default for Pattern {
    fn default() -> Self {
        Self::Brownfield
    }
}

impl Pattern {
    pub fn brownfield() -> Self {
        Self::Brownfield
    }

    pub fn isolation_from_html<B>(html: B) -> Self
    where
        B: Into<Vec<u8>>,
    {
        Self::isolation_from_assets(RuntimeAssets::from_html(html))
    }

    pub fn isolation_from_bytes_map(map: HashMap<String, Vec<u8>>) -> Self {
        Self::isolation_from_assets(RuntimeAssets::from_bytes_map(map))
    }

    pub fn isolation_from_assets(assets: RuntimeAssets) -> Self {
        let schema = format!("isolation-{}", uuid::Uuid::new_v4());
        let key = uuid::Uuid::new_v4().to_string();
        let crypto_keys = RuntimeIsolationKeys::generate();

        Self::Isolation {
            assets: Arc::new(assets),
            schema,
            key,
            crypto_keys: Arc::new(crypto_keys),
        }
    }

    pub fn is_brownfield(&self) -> bool {
        matches!(self, Self::Brownfield)
    }

    pub fn is_isolation(&self) -> bool {
        matches!(self, Self::Isolation { .. })
    }

    pub fn isolation_schema(&self) -> Option<&str> {
        match self {
            Self::Isolation { schema, .. } => Some(schema.as_str()),
            Self::Brownfield => None,
        }
    }

    pub fn isolation_key(&self) -> Option<&str> {
        match self {
            Self::Isolation { key, .. } => Some(key.as_str()),
            Self::Brownfield => None,
        }
    }

    pub fn isolation_assets(&self) -> Option<&RuntimeAssets> {
        match self {
            Self::Isolation { assets, .. } => Some(assets.as_ref()),
            Self::Brownfield => None,
        }
    }

    pub fn isolation_origin(&self, https: bool) -> String {
        match self {
            Self::Isolation { schema, .. } => format_real_schema(schema, https),
            Self::Brownfield => String::new(),
        }
    }
}

/// The shape of the JavaScript Pattern config.
#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase", tag = "pattern")]
pub(crate) enum PatternObject {
    Brownfield,

    Isolation { side: IsolationSide },
}

impl From<&Pattern> for PatternObject {
    fn from(pattern: &Pattern) -> Self {
        match pattern {
            Pattern::Brownfield => Self::Brownfield,

            Pattern::Isolation { .. } => Self::Isolation {
                side: IsolationSide::default(),
            },
        }
    }
}

/// Where the JavaScript is injected to.
#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum IsolationSide {
    /// Original frame, the Brownfield application.
    #[default]
    Original,

    /// Secure frame, the isolation application.
    #[allow(dead_code)]
    Secure,
}

#[derive(Template)]
#[default_template("../../scripts/pattern.js")]
pub(crate) struct PatternJavascript {
    pub(crate) pattern: PatternObject,
}

/// Format the real isolation schema.
///
/// Linux/macOS:
/// `isolation-xxx://localhost/`
///
/// Windows/Android:
/// `http://isolation-xxx.localhost/`
/// or
/// `https://isolation-xxx.localhost/`
pub(crate) fn format_real_schema(schema: &str, https: bool) -> String {
    if cfg!(windows) || cfg!(target_os = "android") {
        let scheme = if https { "https" } else { "http" };
        format!("{scheme}://{schema}.{ISOLATION_IFRAME_SRC_DOMAIN}/")
    } else {
        format!("{schema}://{ISOLATION_IFRAME_SRC_DOMAIN}/")
    }
}

/// Normalize asset paths.
///
/// Examples:
/// ""              -> "index.html"
/// "/"             -> "index.html"
/// "/index.html"   -> "index.html"
/// "assets/app.js" -> "assets/app.js"
fn normalize_asset_path(path: &str) -> String {
    let path = path.trim();

    if path.is_empty() || path == "/" {
        return DEFAULT_ISOLATION_INDEX.to_string();
    }

    path.trim_start_matches('/').replace('\\', "/")
}

/// Simple MIME detection without extra dependency.
fn mime_for_path(path: &str) -> String {
    let lower = path.to_ascii_lowercase();

    if lower.ends_with(".html") || lower.ends_with(".htm") {
        "text/html; charset=utf-8".into()
    } else if lower.ends_with(".js") || lower.ends_with(".mjs") {
        "text/javascript; charset=utf-8".into()
    } else if lower.ends_with(".css") {
        "text/css; charset=utf-8".into()
    } else if lower.ends_with(".json") {
        "application/json; charset=utf-8".into()
    } else if lower.ends_with(".svg") {
        "image/svg+xml".into()
    } else if lower.ends_with(".png") {
        "image/png".into()
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg".into()
    } else if lower.ends_with(".gif") {
        "image/gif".into()
    } else if lower.ends_with(".webp") {
        "image/webp".into()
    } else if lower.ends_with(".ico") {
        "image/x-icon".into()
    } else if lower.ends_with(".wasm") {
        "application/wasm".into()
    } else if lower.ends_with(".txt") {
        "text/plain; charset=utf-8".into()
    } else {
        "application/octet-stream".into()
    }
}
