pub mod async_runtime;
pub mod error;
pub mod events;
pub mod types;
pub mod wrapper;
use crate::{
    protocol::ipc::{IpcRequest, IpcResponse},
    utils::{
        events::{DownloadEvent, PageLoadEvent, SynthesizedEvent, WebviewEvent},
        types::Theme,
    },
};

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::webview::webview::WebviewId;

const MIMETYPE_PLAIN: &str = "text/plain";
use http::Request;
use tao::window::{Theme as TaoTheme, WindowId};
use url::Url;
use wry::{ProxyConfig, ProxyEndpoint, WebContext as WryWebContext};
type ManagerUriSchemeResponderFn =
    Box<dyn FnOnce(http::Response<Cow<'static, [u8]>>) + Send + 'static>;

type ManagerUriSchemeHandler = dyn for<'a> Fn(ManagerUriSchemeContext<'a>, http::Request<Vec<u8>>, ManagerUriSchemeResponder)
    + Send
    + Sync
    + 'static;

/// User-defined IPC bridge.
///
/// The manager only forwards the IPC request.
/// The user decides how commands are routed and handled.
pub type IpcMessageHandler = dyn Fn(IpcRequest) -> IpcResponse + Send + Sync + 'static;
/// Async URI scheme protocol responder.
pub struct ManagerUriSchemeResponder(pub ManagerUriSchemeResponderFn);

impl ManagerUriSchemeResponder {
    /// Resolves the request with the given response.
    pub fn respond<T>(self, response: http::Response<T>)
    where
        T: Into<Cow<'static, [u8]>>,
    {
        let (parts, body) = response.into_parts();
        let response = http::Response::from_parts(parts, body.into());

        (self.0)(response);
    }
}

/// URI scheme protocol context.
#[derive(Clone, Copy, Debug)]
pub struct ManagerUriSchemeContext<'a> {
    window_label: &'a str,
    webview_label: &'a str,
}

impl<'a> ManagerUriSchemeContext<'a> {
    pub fn new(window_label: &'a str, webview_label: &'a str) -> Self {
        Self {
            window_label,
            webview_label,
        }
    }

    /// Get the window label that owns the webview.
    pub fn window_label(&self) -> &'a str {
        self.window_label
    }

    /// Get the webview label that made the URI scheme request.
    pub fn webview_label(&self) -> &'a str {
        self.webview_label
    }
}

/// Uses a custom URI scheme handler to resolve file requests.
pub struct ManagerUriSchemeProtocol {
    handler: Box<ManagerUriSchemeHandler>,
}

impl ManagerUriSchemeProtocol {
    pub fn new<F>(handler: F) -> Self
    where
        F: for<'a> Fn(
                ManagerUriSchemeContext<'a>,
                http::Request<Vec<u8>>,
                ManagerUriSchemeResponder,
            ) + Send
            + Sync
            + 'static,
    {
        Self {
            handler: Box::new(handler),
        }
    }

    pub fn handle(
        &self,
        context: ManagerUriSchemeContext<'_>,
        request: http::Request<Vec<u8>>,
        responder: ManagerUriSchemeResponder,
    ) {
        (self.handler)(context, request, responder);
    }
}

pub(crate) fn parse_proxy_url(url: &Url) -> crate::Result<ProxyConfig> {
    let host = url.host().map(|h| h.to_string()).unwrap_or_default();
    let port = url.port().map(|p| p.to_string()).unwrap_or_default();

    if url.scheme() == "http" {
        let config = ProxyConfig::Http(ProxyEndpoint { host, port });

        Ok(config)
    } else if url.scheme() == "socks5" {
        let config = ProxyConfig::Socks5(ProxyEndpoint { host, port });

        Ok(config)
    } else {
        Err(crate::Error::InvalidProxyUrl)
    }
}

pub(crate) fn is_local_network_url(url: &url::Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(s)) => s == "localhost",
        Some(url::Host::Ipv4(_)) | Some(url::Host::Ipv6(_)) => true,
        None => false,
    }
}
#[derive(Debug)]
pub struct WebContext {
    pub inner: WryWebContext,
    pub referenced_by_webviews: HashSet<String>,
    // on Linux the custom protocols are associated with the context
    // and you cannot register a URI scheme more than once
    pub registered_custom_protocols: HashSet<String>,
}

pub type WebContextStore = Arc<Mutex<HashMap<Option<PathBuf>, WebContext>>>;
pub type WebviewIpcHandler = Box<dyn Fn(WebViewMetaData, Request<String>) + Send>;

pub type WebviewEventId = u32;
pub type WebviewEventHandler = Box<dyn Fn(&WebviewEvent) + Send>;
pub type WebviewEventListeners = Arc<Mutex<HashMap<WebviewEventId, WebviewEventHandler>>>;
#[cfg(target_os = "android")]
pub struct CreationContext<'a, 'b> {
    pub env: &'a mut jni::JNIEnv<'b>,
    pub activity: &'a jni::objects::JObject<'b>,
    pub webview: &'a jni::objects::JObject<'b>,
}

pub type IpcHandler = dyn Fn(Request<String>) + 'static;

pub type UriSchemeProtocolHandler = dyn Fn(&str, http::Request<Vec<u8>>, Box<dyn FnOnce(http::Response<Cow<'static, [u8]>>) + Send>)
    + Send
    + Sync
    + 'static;

pub type ProxyHandler = Arc<dyn Fn(WindowId, WebviewId, SynthesizedEvent) + Send + Sync + 'static>;
pub type WebResourceRequestHandler =
    dyn Fn(http::Request<Vec<u8>>, &mut http::Response<Cow<'static, [u8]>>) + Send + Sync;

pub type NavigationHandler = dyn Fn(&Url) -> bool + Send;

pub type NewWindowHandler = dyn Fn(Url, NewWindowFeatures) -> NewWindowResponse + Send;

pub type OnPageLoadHandler = dyn Fn(Url, PageLoadEvent) + Send;

pub type DocumentTitleChangedHandler = dyn Fn(String) + Send + 'static;

pub type DownloadHandler = dyn Fn(DownloadEvent) -> bool + Send + Sync;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub type OnWebContentProcessTerminateHandler = dyn Fn() + Send;

#[cfg(target_os = "ios")]
pub type InputAccessoryViewBuilderFn = dyn Fn(&objc2_ui_kit::UIView) -> Option<objc2::rc::Retained<objc2_ui_kit::UIView>>
    + Send
    + Sync
    + 'static;

/// Information about the webview that initiated a new window request.
#[derive(Debug)]
pub struct NewWindowOpener {
    /// The instance of the webview that initiated the new window request.
    ///
    /// This must be set as the related view of the new webview. See [`WebviewAttributes::related_view`].
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
    ))]
    pub webview: webkit2gtk::WebView,
    /// The instance of the webview that initiated the new window request.
    ///
    /// The target webview environment **MUST** match the environment of the opener webview. See [`WebviewAttributes::with_environment`].
    #[cfg(windows)]
    pub webview: webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2,
    #[cfg(windows)]
    pub environment: webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Environment,
    /// The instance of the webview that initiated the new window request.
    #[cfg(target_os = "macos")]
    pub webview: objc2::rc::Retained<objc2_web_kit::WKWebView>,
    /// Configuration of the target webview.
    ///
    /// This **MUST** be used when creating the target webview. See [`WebviewAttributes::webview_configuration`].
    #[cfg(target_os = "macos")]
    pub target_configuration: objc2::rc::Retained<objc2_web_kit::WKWebViewConfiguration>,
}

/// Window features of a window requested to open.
#[derive(Debug)]
pub struct NewWindowFeatures {
    pub(crate) size: Option<dpi::LogicalSize<f64>>,
    pub(crate) position: Option<dpi::LogicalPosition<f64>>,
    pub(crate) opener: NewWindowOpener,
}

impl NewWindowFeatures {
    pub fn new(
        size: Option<dpi::LogicalSize<f64>>,
        position: Option<dpi::LogicalPosition<f64>>,
        opener: NewWindowOpener,
    ) -> Self {
        Self {
            size,
            position,
            opener,
        }
    }

    /// Specifies the size of the content area
    /// as defined by the user's operating system where the new window will be generated.
    pub fn size(&self) -> Option<dpi::LogicalSize<f64>> {
        self.size
    }

    /// Specifies the position of the window relative to the work area
    /// as defined by the user's operating system where the new window will be generated.
    pub fn position(&self) -> Option<dpi::LogicalPosition<f64>> {
        self.position
    }

    /// Returns information about the webview that initiated a new window request.
    pub fn opener(&self) -> &NewWindowOpener {
        &self.opener
    }
}

/// Response for the new window request handler.
pub enum NewWindowResponse {
    /// Allow the window to be opened with the default implementation.
    Allow,
    /// Allow the window to be opened, with the given window.
    ///
    /// ## Platform-specific:
    ///
    /// **Linux**: The webview must be related to the caller webview. See [`WebviewAttributes::related_view`].
    /// **Windows**: The webview must use the same environment as the caller webview. See [`WebviewAttributes::with_environment`].
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    Create { window_id: WindowId },
    /// Deny the window from being opened.
    Deny,
}

#[derive(Debug, Clone)]
pub(crate) struct WebviewBounds {
    pub x_rate: f32,
    pub y_rate: f32,
    pub width_rate: f32,
    pub height_rate: f32,
}

/// URI scheme protocol context.
#[derive(Clone, Copy, Debug)]
pub struct WebViewMetaData<'a> {
    is_kind: &'a bool,

    window_label: &'a str,
    webview_label: &'a str,

    window_id: &'a WindowId,
    webview_id: &'a WebviewId,
}

impl<'a> WebViewMetaData<'a> {
    pub fn new(
        is_kind: &'a bool,
        window_label: &'a str,
        webview_label: &'a str,
        window_id: &'a WindowId,
        webview_id: &'a WebviewId,
    ) -> Self {
        Self {
            is_kind,
            window_label,
            webview_label,
            window_id,
            webview_id,
        }
    }

    /// Returns whether this webview is a child/kind webview.
    pub fn is_kind(&self) -> &'a bool {
        self.is_kind
    }

    /// Returns the window label that owns the webview.
    pub fn window_label(&self) -> &'a str {
        self.window_label
    }

    /// Returns the webview label that made the request.
    pub fn webview_label(&self) -> &'a str {
        self.webview_label
    }

    /// Returns the window id that owns the webview.
    pub fn window_id(&self) -> &'a WindowId {
        self.window_id
    }

    /// Returns the webview id.
    pub fn webview_id(&self) -> &'a WebviewId {
        self.webview_id
    }
}

/// [Web Compatible MimeTypes](https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types#important_mime_types_for_web_developers)
#[allow(missing_docs)]
pub enum MimeType {
    Css,
    Csv,
    Html,
    Ico,
    Js,
    Json,
    Jsonld,
    Mp4,
    OctetStream,
    Rtf,
    Svg,
    Txt,
}

impl std::fmt::Display for MimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mime = match self {
            MimeType::Css => "text/css",
            MimeType::Csv => "text/csv",
            MimeType::Html => "text/html",
            MimeType::Ico => "image/vnd.microsoft.icon",
            MimeType::Js => "text/javascript",
            MimeType::Json => "application/json",
            MimeType::Jsonld => "application/ld+json",
            MimeType::Mp4 => "video/mp4",
            MimeType::OctetStream => "application/octet-stream",
            MimeType::Rtf => "application/rtf",
            MimeType::Svg => "image/svg+xml",
            MimeType::Txt => MIMETYPE_PLAIN,
        };
        write!(f, "{mime}")
    }
}

impl MimeType {
    /// parse a URI suffix to convert text/plain mimeType to their actual web compatible mimeType.
    pub fn parse_from_uri(uri: &str) -> MimeType {
        Self::parse_from_uri_with_fallback(uri, Self::Html)
    }

    /// parse a URI suffix to convert text/plain mimeType to their actual web compatible mimeType with specified fallback for unknown file extensions.
    pub fn parse_from_uri_with_fallback(uri: &str, fallback: MimeType) -> MimeType {
        let suffix = uri.split('.').next_back();
        match suffix {
            Some("bin") => Self::OctetStream,
            Some("css" | "less" | "sass" | "styl") => Self::Css,
            Some("csv") => Self::Csv,
            Some("html") => Self::Html,
            Some("ico") => Self::Ico,
            Some("js") => Self::Js,
            Some("json") => Self::Json,
            Some("jsonld") => Self::Jsonld,
            Some("mjs") => Self::Js,
            Some("mp4") => Self::Mp4,
            Some("rtf") => Self::Rtf,
            Some("svg") => Self::Svg,
            Some("txt") => Self::Txt,
            // Assume HTML when a TLD is found for eg. `wry:://tauri.app` | `wry://hello.com`
            Some(_) => fallback,
            // using octet stream according to this:
            // <https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types/Common_types>
            None => Self::OctetStream,
        }
    }

    /// infer mimetype from content (or) URI if needed.
    pub fn parse(content: &[u8], uri: &str) -> String {
        Self::parse_with_fallback(content, uri, Self::Html)
    }
    /// infer mimetype from content (or) URI if needed with specified fallback for unknown file extensions.
    pub fn parse_with_fallback(content: &[u8], uri: &str, fallback: MimeType) -> String {
        let mime = if uri.ends_with(".svg") {
            // when reading svg, we can't use `infer`
            None
        } else {
            infer::get(content).map(|info| info.mime_type())
        };

        match mime {
            Some(mime) if mime == MIMETYPE_PLAIN => {
                Self::parse_from_uri_with_fallback(uri, fallback).to_string()
            }
            None => Self::parse_from_uri_with_fallback(uri, fallback).to_string(),
            Some(mime) => mime.to_string(),
        }
    }
}

/* #[cfg(target_os = "macos")]
fn inner_size(
  window: &Window,
  webviews: &[Webview],
  has_children: bool,
) -> TaoPhysicalSize<u32> {
  if !has_children && !webviews.is_empty() {
    use wry::WebViewExtMacOS;
    let webview = webviews.first().unwrap();
    let view = unsafe { Retained::cast_unchecked::<objc2_app_kit::NSView>(webview.webview()) };
    let view_frame = view.frame();
    let logical: TaoLogicalSize<f64> = (view_frame.size.width, view_frame.size.height).into();
    return logical.to_physical(window.scale_factor());
  }

  window.inner_size()
}

#[cfg(not(target_os = "macos"))]
#[allow(unused_variables)]
fn inner_size(
  window: &Window,
  webviews: &[Webview],
  has_children: bool,
) -> TaoPhysicalSize<u32> {
  window.inner_size()
} */

pub(crate) fn to_tao_theme(theme: Option<Theme>) -> Option<TaoTheme> {
    match theme {
        Some(Theme::Light) => Some(TaoTheme::Light),
        Some(Theme::Dark) => Some(TaoTheme::Dark),
        _ => None,
    }
}
