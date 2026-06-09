use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU32, Ordering},
    },
};

use dpi::{LogicalPosition, LogicalSize};
use tao::window::{Window, WindowId};
use url::Url;

use crate::{
    factory::create_wry_webview,
    pending::PendingWebview,
    protocol,
    types::{FrontendDist, WebviewUrl},
    utils::{
        ManagerUriSchemeContext, ManagerUriSchemeProtocol, ManagerUriSchemeResponder,
        WebContextStore, is_local_network_url,
    },
    webview::{WebView, WebviewId},
};

const APP_PROTOCOL: &str = "taurino";
const IPC_PROTOCOL: &str = "ipc";

pub struct Manager {
    window_label: Arc<str>,
    pub window_id: Arc<Mutex<Option<WindowId>>>,

    pub webviews: Arc<Mutex<HashMap<String, WebView>>>,

    uri_scheme_protocols: Mutex<HashMap<String, Arc<ManagerUriSchemeProtocol>>>,
    /// Custom protocols to register on the webview
    frontend_dist: Option<FrontendDist>,
    web_context: WebContextStore,

    next_webview_id: Arc<AtomicU32>,
    next_webview_event_id: Arc<AtomicU32>,

    pub webview_runtime_installed: bool,
}
impl Manager {
    pub fn new() -> crate::Result<Self> {
        Ok(Self {
            window_label: Arc::from("root"),
            window_id: Arc::new(Mutex::new(None)),
            webviews: Arc::new(Mutex::new(HashMap::new())),
            uri_scheme_protocols: Mutex::new(HashMap::new()),
            frontend_dist: None,
            web_context: Default::default(),
            next_webview_id: Arc::new(AtomicU32::new(0)),
            next_webview_event_id: Arc::new(AtomicU32::new(0)),
            webview_runtime_installed: wry::webview_version().is_ok(),
        })
    }

    pub fn set_window_label<S>(mut self, window_label: S) -> Self
    where
        S: Into<Arc<str>>,
    {
        self.window_label = window_label.into();
        self
    }

    pub fn set_window_id(self, window_id: WindowId) -> Self {
        *self.window_id.lock().expect("poisoned window id manager") = Some(window_id);

        self
    }

    pub fn with_dev_server_url(mut self, url: &str) -> crate::Result<Self> {
        let url = Url::parse(url).map_err(crate::Error::InvalidUrl)?;
        self.frontend_dist = Some(FrontendDist::Url(url));
        Ok(self)
    }

    pub fn set_static_dir<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.frontend_dist = Some(FrontendDist::Directory(path.into()));
        self
    }
    pub fn set_frontend_dist(mut self, path: FrontendDist) -> Self {
        self.frontend_dist = Some(path);
        self
    }

    pub fn set_static_files(mut self, files: Vec<PathBuf>) -> Self {
        self.frontend_dist = Some(FrontendDist::Files(files));
        self
    }

    pub(crate) fn next_webview_id(&self) -> WebviewId {
        self.next_webview_id.fetch_add(1, Ordering::Relaxed)
    }

    pub(crate) fn next_webview_event_id(&self) -> u32 {
        self.next_webview_event_id.fetch_add(1, Ordering::Relaxed)
    }
    #[allow(dead_code)]
    fn webview_runtime_installed(&self) -> bool {
        self.webview_runtime_installed
    }

    pub(crate) fn web_context(&self) -> &WebContextStore {
        &self.web_context
    }

    /// Get a locked handle to the webviews.
    pub(crate) fn webviews_lock(&self) -> MutexGuard<'_, HashMap<String, WebView>> {
        self.webviews.lock().expect("poisoned webview manager")
    }

    pub(crate) fn webviews_store(&self) -> Arc<Mutex<HashMap<String, WebView>>> {
        Arc::clone(&self.webviews)
    }

    /// Get the base app URL.
    ///
    /// - If `frontend_dist` is an URL, this URL is returned.
    /// - Otherwise the custom app protocol URL is returned.
    pub(crate) fn get_app_url(&self, https: bool) -> Cow<'_, Url> {
        match self.frontend_dist.as_ref() {
            Some(FrontendDist::Url(url)) => Cow::Borrowed(url),
            _ => self.taurino_protocol_url(https),
        }
    }

    /// The custom protocol URL used to serve embedded assets.
    ///
    /// Returns:
    /// - `taurino://localhost`
    /// - or on Windows/Android:
    ///   - `http://taurino.localhost`
    ///   - `https://taurino.localhost`
    pub(crate) fn taurino_protocol_url(&self, https: bool) -> Cow<'_, Url> {
        if cfg!(windows) || cfg!(target_os = "android") {
            let scheme = if https { "https" } else { "http" };
            let url = format!("{scheme}://{APP_PROTOCOL}.localhost");

            Cow::Owned(Url::parse(&url).expect("Invalid taurino localhost URL"))
        } else {
            Cow::Owned(
                Url::parse(&format!("{APP_PROTOCOL}://localhost"))
                    .expect("Invalid taurino protocol URL"),
            )
        }
    }

    pub fn register_uri_scheme_protocol<N>(
        &self,
        uri_scheme: N,
        protocol: Arc<ManagerUriSchemeProtocol>,
    ) where
        N: Into<String>,
    {
        let uri_scheme = uri_scheme.into();

        self.uri_scheme_protocols
            .lock()
            .expect("poisoned URI scheme protocol manager")
            .insert(uri_scheme, protocol);
    }

    fn prepare_webview(&mut self, mut pending: PendingWebview) -> crate::Result<PendingWebview> {
        if self.webviews_lock().contains_key(&pending.label) {
            return Err(crate::Error::WebviewLabelAlreadyExists(pending.label));
        }

        let label = pending.label.clone();
        #[allow(unused_mut)] // mut url only for the data-url parsing
        let mut url = match &pending.webview_attributes.url {
            WebviewUrl::App(path) => {
                let app_url = self.get_app_url(pending.webview_attributes.use_https_scheme);
                let url = if is_local_network_url(&app_url) {
                    Cow::Owned(Url::parse("taurino://localhost").unwrap())
                } else {
                    app_url
                };
                // ignore "index.html" just to simplify the url
                if path.to_str() != Some("index.html") {
                    url.join(&path.to_string_lossy())
                        .map_err(crate::Error::InvalidUrl)
                        // this will never fail
                        .unwrap()
                } else {
                    url.into_owned()
                }
            }
            WebviewUrl::External(url) => {
                let config_url = self.get_app_url(pending.webview_attributes.use_https_scheme);
                let is_app_url = config_url.make_relative(&url).is_some();
                let mut url = url.clone();
                if is_app_url && is_local_network_url(&url) {
                    Url::parse("taurino://localhost").unwrap()
                } else {
                    url
                }
            }

            WebviewUrl::CustomProtocol(url) => url.clone(),
            #[allow(unreachable_patterns)]
            _ => unimplemented!(),
        };

        if url.scheme() == "data" {
            if let Ok(data_url) = data_url::DataUrl::process(url.as_str()) {
                let (body, _) = data_url.decode_to_vec().unwrap();
                let html = String::from_utf8_lossy(&body).into_owned();
                // naive way to check if it's an html
                if html.contains('<') && html.contains('>') {
                    url.set_path(&format!("{},{html}", mime::TEXT_HTML));
                }
            }
        }

        pending.url = url.to_string();

        self.prepare_pending_webview(pending, &label)
    }

    fn prepare_pending_webview(
        &mut self,
        mut pending: PendingWebview,
        label: &str,
    ) -> crate::Result<PendingWebview> {
        let use_https_scheme = pending.webview_attributes.use_https_scheme;

        let protocols = self.registered_uri_scheme_protocols();
        let mut registered_scheme_protocols = HashSet::new();

        for (uri_scheme, protocol) in protocols {
            registered_scheme_protocols.insert(uri_scheme.clone());

            let window_label = Arc::clone(&self.window_label);

            pending.internal_register_uri_scheme_protocol(
                uri_scheme,
                move |webview_id, request, responder| {
                    let context = ManagerUriSchemeContext::new(window_label.as_ref(), webview_id);

                    protocol.handle(context, request, ManagerUriSchemeResponder(responder));
                },
            );
        }

        let window_url = Url::parse(&pending.url).map_err(crate::Error::InvalidUrl)?;

        let window_origin = Self::window_origin(&window_url, use_https_scheme);

        self.register_builtin_protocols(
            &mut pending,
            &mut registered_scheme_protocols,
            window_origin,
        );

        let _ = label;

        Ok(pending)
    }

    fn registered_uri_scheme_protocols(&self) -> Vec<(String, Arc<ManagerUriSchemeProtocol>)> {
        self.uri_scheme_protocols
            .lock()
            .expect("poisoned URI scheme protocol manager")
            .iter()
            .map(|(scheme, protocol)| (scheme.clone(), Arc::clone(protocol)))
            .collect()
    }

    fn register_builtin_protocols(
        &mut self,
        pending: &mut PendingWebview,
        registered_scheme_protocols: &mut HashSet<String>,
        window_origin: String,
    ) {
        if !registered_scheme_protocols.contains(APP_PROTOCOL) {
            let _web_resource_request_handler = pending.web_resource_request_handler.take();

            let app_protocol = protocol::get(self.frontend_dist.clone(), &window_origin);

            let window_label = Arc::clone(&self.window_label);

            pending.internal_register_uri_scheme_protocol(
                APP_PROTOCOL,
                move |webview_id, request, responder| {
                    let context = ManagerUriSchemeContext::new(window_label.as_ref(), webview_id);

                    app_protocol.handle(context, request, ManagerUriSchemeResponder(responder));
                },
            );

            registered_scheme_protocols.insert(APP_PROTOCOL.to_string());
        }

        if !registered_scheme_protocols.contains(IPC_PROTOCOL) {
            pending.internal_register_uri_scheme_protocol(
                IPC_PROTOCOL,
                move |_webview_id, _request, _responder| {
                    // TODO:
                    // Hier später dein IPC protocol handler.
                },
            );

            registered_scheme_protocols.insert(IPC_PROTOCOL.to_string());
        }
    }

    fn window_origin(url: &Url, use_https_scheme: bool) -> String {
        if url.scheme() == "data" {
            return "null".into();
        }

        if (cfg!(windows) || cfg!(target_os = "android"))
            && url.scheme() != "http"
            && url.scheme() != "https"
        {
            let scheme = if use_https_scheme { "https" } else { "http" };
            return format!("{scheme}://{}.localhost", url.scheme());
        }

        if let Some(host) = url.host() {
            let port = url
                .port()
                .map(|port| format!(":{port}"))
                .unwrap_or_default();

            return format!("{}://{}{}", url.scheme(), host, port);
        }

        "null".into()
    }
    pub fn resize_webviews(&self, window: &Window, size: tao::dpi::PhysicalSize<u32>) {
        let size = size.to_logical::<f32>(window.scale_factor());

        let webviews = self.webviews_lock();

        for webview in webviews.values() {
            let bounds = webview
                .bounds
                .lock()
                .expect("poisoned webview bounds")
                .clone();

            let Some(b) = bounds else {
                continue;
            };

            if let Err(e) = webview.set_bounds(wry::Rect {
                position: LogicalPosition::new(size.width * b.x_rate, size.height * b.y_rate)
                    .into(),
                size: LogicalSize::new(size.width * b.width_rate, size.height * b.height_rate)
                    .into(),
            }) {
                eprintln!("failed to autoresize webview: {e}");
            }
        }
    }
    pub fn create_webview(
        &mut self,
        window: &Window,
        pending: PendingWebview,
    ) -> crate::Result<()> {
        let pending = self.prepare_webview(pending)?;

        let webview = create_wry_webview(self.window_label.to_string(), window, pending, self)?;

        self.webviews_lock().insert(webview.label.clone(), webview);

        Ok(())
    }
}
