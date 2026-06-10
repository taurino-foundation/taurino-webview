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
use serialize_to_javascript::{DefaultTemplate, Template, default_template};
use tao::window::{Window, WindowId};
use url::Url;

use crate::{
    attributes::InitializationScript,
    factory::create_wry_webview,
    layout::FixedLayout,
    pending::PendingWebview,
    protocol,
    types::{FrontendDist, WebviewUrl},
    utils::{
        ManagerUriSchemeContext, ManagerUriSchemeProtocol,
        ManagerUriSchemeResponder, WebContextStore, is_local_network_url,
    },
    webview::{WebView, WebviewId},
};

const APP_PROTOCOL: &str = "taurino";
const IPC_PROTOCOL: &str = "ipc";

#[allow(dead_code)]
pub(crate) const PROCESS_IPC_MESSAGE_FN: &str =
    include_str!("../scripts/process-ipc-message-fn.js");

#[allow(dead_code)]
#[derive(Template)]
#[default_template("../scripts/isolation.js")]
pub(crate) struct IsolationJavascript<'a> {
    pub(crate) isolation_src: &'a str,
    pub(crate) style: &'a str,
}

#[allow(dead_code)]
#[derive(Template)]
#[default_template("../scripts/ipc.js")]
pub(crate) struct IpcJavascript<'a> {
    pub(crate) isolation_origin: &'a str,
}

pub struct ManagerConfig {
    freeze_prototype: bool,
    pattern: bool,

    pub webview_runtime_installed: bool,
    /// The script that initializes the invoke system.
    pub invoke_initialization_script: String,

    /// A runtime generated invoke key.
    pub(crate) invoke_key: String,
    /// Custom protocols to register on the webview.
    frontend_dist: Option<FrontendDist>,
    pub web_context: WebContextStore,
}
impl ManagerConfig {
    pub fn new() -> crate::Result<Self> {
        Ok(Self::default())
    }

    // ---------------------------------------------------------------------
    // Getter
    // ---------------------------------------------------------------------

    pub fn freeze_prototype(&self) -> bool {
        self.freeze_prototype
    }

    pub fn pattern(&self) -> bool {
        self.pattern
    }

    pub fn webview_runtime_installed(&self) -> bool {
        self.webview_runtime_installed
    }

    pub fn invoke_initialization_script(&self) -> &str {
        &self.invoke_initialization_script
    }
    #[allow(dead_code)]
    pub(crate) fn invoke_key(&self) -> &str {
        &self.invoke_key
    }

    pub fn frontend_dist(&self) -> Option<&FrontendDist> {
        self.frontend_dist.as_ref()
    }

    pub(crate) fn resource_path(&self) -> Option<&FrontendDist> {
        self.frontend_dist.as_ref()
    }

    pub(crate) fn web_context(&self) -> &WebContextStore {
        &self.web_context
    }

    // ---------------------------------------------------------------------
    // Builder-Setter
    // ---------------------------------------------------------------------

    #[allow(dead_code)]
    pub fn set_freeze_prototype(mut self, value: bool) -> Self {
        self.freeze_prototype = value;
        self
    }

    #[allow(dead_code)]
    pub fn set_pattern(mut self, value: bool) -> Self {
        self.pattern = value;
        self
    }

    #[allow(dead_code)]
    pub fn set_webview_runtime_installed(mut self, value: bool) -> Self {
        self.webview_runtime_installed = value;
        self
    }

    #[allow(dead_code)]
    pub fn set_invoke_initialization_script<S>(mut self, script: S) -> Self
    where
        S: Into<String>,
    {
        self.invoke_initialization_script = script.into();
        self
    }

    #[allow(dead_code)]
    pub(crate) fn set_invoke_key<S>(mut self, key: S) -> Self
    where
        S: Into<String>,
    {
        self.invoke_key = key.into();
        self
    }

    #[allow(dead_code)]
    pub fn set_web_context(mut self, web_context: WebContextStore) -> Self {
        self.web_context = web_context;
        self
    }

    #[allow(dead_code)]
    pub fn set_frontend_dist(mut self, frontend_dist: FrontendDist) -> Self {
        self.frontend_dist = Some(frontend_dist);
        self
    }

    #[allow(dead_code)]
    pub fn clear_frontend_dist(mut self) -> Self {
        self.frontend_dist = None;
        self
    }

    // ---------------------------------------------------------------------
    // Convenience-Setter für FrontendDist
    // ---------------------------------------------------------------------

    pub fn with_dev_server_url(mut self, url: &str) -> crate::Result<Self> {
        let url = Url::parse(url).map_err(crate::Error::InvalidUrl)?;
        self.frontend_dist = Some(FrontendDist::Url(url));
        Ok(self)
    }

    #[allow(dead_code)]
    pub fn set_static_dir<P>(mut self, path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        self.frontend_dist = Some(FrontendDist::Directory(path.into()));
        self
    }

    #[allow(dead_code)]
    pub fn set_static_files(mut self, files: Vec<PathBuf>) -> Self {
        self.frontend_dist = Some(FrontendDist::Files(files));
        self
    }
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            freeze_prototype: false,
            pattern: false,
            webview_runtime_installed: wry::webview_version().is_ok(),
            invoke_initialization_script: Default::default(),
            invoke_key: Default::default(),
            frontend_dist: None,
            web_context: Default::default(),
        }
    }
}
pub struct Manager {
    pub(crate) config: ManagerConfig,
    pub(crate) window_label: Arc<str>,
    pub window_id: Arc<Mutex<Option<WindowId>>>,

    pub webviews: Arc<Mutex<HashMap<String, WebView>>>,

    pub(crate) uri_scheme_protocols:
        Mutex<HashMap<String, Arc<ManagerUriSchemeProtocol>>>,

    next_webview_id: Arc<AtomicU32>,
    next_webview_event_id: Arc<AtomicU32>,
}

impl Manager {
    pub fn new() -> crate::Result<Self> {
        Ok(Self {
            config: ManagerConfig::default(),
            window_label: Arc::from("main"),
            window_id: Arc::new(Mutex::new(None)),
            webviews: Arc::new(Mutex::new(HashMap::new())),
            uri_scheme_protocols: Mutex::new(HashMap::new()),
            next_webview_id: Arc::new(AtomicU32::new(0)),
            next_webview_event_id: Arc::new(AtomicU32::new(0)),
        })
    }

    pub fn set_manager_config(mut self, config: ManagerConfig) -> Self {
        self.config = config.into();
        self
    }

    pub fn set_window_label<S>(mut self, window_label: S) -> Self
    where
        S: Into<Arc<str>>,
    {
        self.window_label = window_label.into();
        self
    }

    pub fn set_window_id(self, window_id: WindowId) -> Self {
        *self.window_id.lock().expect("poisoned window id manager") =
            Some(window_id);

        self
    }

    pub(crate) fn next_webview_id(&self) -> WebviewId {
        self.next_webview_id.fetch_add(1, Ordering::Relaxed)
    }

    pub(crate) fn next_webview_event_id(&self) -> u32 {
        self.next_webview_event_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Get a locked handle to the webviews.
    pub(crate) fn webviews_lock(
        &self,
    ) -> MutexGuard<'_, HashMap<String, WebView>> {
        self.webviews.lock().expect("poisoned webview manager")
    }

    pub(crate) fn webviews_store(
        &self,
    ) -> Arc<Mutex<HashMap<String, WebView>>> {
        Arc::clone(&self.webviews)
    }

    /// Get the base app URL.
    ///
    /// - If `frontend_dist` is an URL, this URL is returned.
    /// - Otherwise the custom app protocol URL is returned.
    pub(crate) fn get_app_url(&self, https: bool) -> Cow<'_, Url> {
        match self.config.resource_path() {
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

            Cow::Owned(Url::parse(&url).expect("invalid taurino localhost URL"))
        } else {
            Cow::Owned(
                Url::parse(&format!("{APP_PROTOCOL}://localhost"))
                    .expect("invalid taurino protocol URL"),
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

    fn prepare_webview(
        &mut self,
        mut pending: PendingWebview,
    ) -> crate::Result<PendingWebview> {
        if self.webviews_lock().contains_key(&pending.label) {
            return Err(crate::Error::WebviewLabelAlreadyExists(pending.label));
        }

        let label = pending.label.clone();

        let mut url = match &pending.webview_attributes.url {
            WebviewUrl::App(path) => {
                let app_url = self
                    .get_app_url(pending.webview_attributes.use_https_scheme);

                let url = if is_local_network_url(&app_url) {
                    Cow::Owned(
                        Url::parse("taurino://localhost")
                            .expect("invalid app URL"),
                    )
                } else {
                    app_url
                };

                // Ignore `index.html` just to simplify the URL.
                if path.to_str() != Some("index.html") {
                    url.join(&path.to_string_lossy())
                        .map_err(crate::Error::InvalidUrl)?
                } else {
                    url.into_owned()
                }
            }
            WebviewUrl::External(url) => {
                let config_url = self
                    .get_app_url(pending.webview_attributes.use_https_scheme);
                let is_app_url = config_url.make_relative(url).is_some();
                let url = url.clone();

                if is_app_url && is_local_network_url(&url) {
                    Url::parse("taurino://localhost").expect("invalid app URL")
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
                if let Ok((body, _)) = data_url.decode_to_vec() {
                    let html = String::from_utf8_lossy(&body).into_owned();

                    // Naive way to check if it is HTML.
                    if html.contains('<') && html.contains('>') {
                        url.set_path(&format!(",{html}"));
                    }
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

        let mut all_initialization_scripts: Vec<InitializationScript> =
            Vec::new();

        fn main_frame_script(script: String) -> InitializationScript {
            InitializationScript {
                script,
                for_main_frame_only: true,
            }
        }

        all_initialization_scripts.push(main_frame_script(
            r#"
            Object.defineProperty(window, 'isTaurino', {
                value: true,
            });

            if (!window.__TAURINO_INTERNALS__) {
                Object.defineProperty(window, '__TAURINO_INTERNALS__', {
                    value: {
                        plugins: {}
                    }
                });
            }
            "#
            .to_owned(),
        ));

        all_initialization_scripts.push(main_frame_script(format!(
            r#"
            Object.defineProperty(window.__TAURINO_INTERNALS__, 'metadata', {{
                value: {{
                    currentWindow: {{ label: {current_window_label} }},
                    currentWebview: {{ label: {current_webview_label} }}
                }}
            }});
            "#,
            current_window_label =
                serde_json::to_string(self.window_label.as_ref())?,
            current_webview_label = serde_json::to_string(label)?,
        )));

        let ipc_script = "";
        let pattern_script = "";

        all_initialization_scripts.push(main_frame_script(
            self.initialization_script(
                ipc_script,
                pattern_script,
                use_https_scheme,
            )?,
        ));

        pending
            .webview_attributes
            .initialization_scripts
            .extend(all_initialization_scripts);

        let protocols = self.registered_uri_scheme_protocols();
        let mut registered_scheme_protocols = HashSet::new();

        for (uri_scheme, protocol) in protocols {
            registered_scheme_protocols.insert(uri_scheme.clone());

            let window_label = Arc::clone(&self.window_label);

            pending.internal_register_uri_scheme_protocol(
                uri_scheme,
                move |webview_id, request, responder| {
                    let context = ManagerUriSchemeContext::new(
                        window_label.as_ref(),
                        webview_id,
                    );

                    protocol.handle(
                        context,
                        request,
                        ManagerUriSchemeResponder(responder),
                    );
                },
            );
        }

        let window_url =
            Url::parse(&pending.url).map_err(crate::Error::InvalidUrl)?;
        let window_origin = Self::window_origin(&window_url, use_https_scheme);

        self.register_builtin_protocols(
            &mut pending,
            &mut registered_scheme_protocols,
            window_origin,
        );

        Ok(pending)
    }

    fn registered_uri_scheme_protocols(
        &self,
    ) -> Vec<(String, Arc<ManagerUriSchemeProtocol>)> {
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
            let _web_resource_request_handler =
                pending.web_resource_request_handler.take();

            let app_protocol = protocol::get(
                self.config.resource_path().cloned(),
                &window_origin,
            );
            let window_label = Arc::clone(&self.window_label);

            pending.internal_register_uri_scheme_protocol(
                APP_PROTOCOL,
                move |webview_id, request, responder| {
                    let context = ManagerUriSchemeContext::new(
                        window_label.as_ref(),
                        webview_id,
                    );

                    app_protocol.handle(
                        context,
                        request,
                        ManagerUriSchemeResponder(responder),
                    );
                },
            );

            registered_scheme_protocols.insert(APP_PROTOCOL.to_string());
        }

        if !registered_scheme_protocols.contains(IPC_PROTOCOL) {
            pending.internal_register_uri_scheme_protocol(
                IPC_PROTOCOL,
                move |_webview_id, _request, _responder| {
                    // TODO: Add your IPC protocol handler here.
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

    fn event_initialization_script(
        function_name: &str,
        listeners: &str,
    ) -> String {
        format!(
            r#"
            Object.defineProperty(window, '{function_name}', {{
                value: function (eventData, ids) {{
                    const listeners = (window['{listeners}'] && window['{listeners}'][eventData.event]) || [];

                    for (const id of ids) {{
                        const listener = listeners[id];

                        if (listener) {{
                            eventData.id = id;

                            if (
                                window.__TAURINO_INTERNALS__ &&
                                typeof window.__TAURINO_INTERNALS__.runCallback === 'function'
                            ) {{
                                window.__TAURINO_INTERNALS__.runCallback(listener.handlerId, eventData);
                            }}
                        }}
                    }}
                }}
            }});
            "#
        )
    }

    fn initialization_script(
        &self,
        ipc_script: &str,
        pattern_script: &str,
        use_https_scheme: bool,
    ) -> crate::Result<String> {
        #[derive(Template)]
        #[default_template("../scripts/init.js")]
        struct InitJavascript<'a> {
            #[raw]
            pattern_script: &'a str,
            #[raw]
            ipc_script: &'a str,
            #[raw]
            core_script: &'a str,
            #[raw]
            event_initialization_script: &'a str,
            #[raw]
            freeze_prototype: &'a str,
        }

        #[derive(Template)]
        #[default_template("../scripts/core.js")]
        struct CoreJavascript<'a> {
            os_name: &'a str,
            protocol_scheme: &'a str,
            invoke_key: &'a str,
        }

        let core_script = CoreJavascript {
            os_name: std::env::consts::OS,
            protocol_scheme: if use_https_scheme { "https" } else { "http" },
            invoke_key: "",
        }
        .render_default(&Default::default())?
        .into_string();

        let event_script = Self::event_initialization_script(
            "__TAURINO_EVENT__",
            "__TAURINO_LISTENERS__",
        );

        InitJavascript {
            pattern_script,
            ipc_script,
            core_script: &core_script,
            event_initialization_script: &event_script,
            freeze_prototype: "",
        }
        .render_default(&Default::default())
        .map(|script| script.into_string())
        .map_err(Into::into)
    }

    pub fn resize_webviews(
        &self,
        window: &Window,
        size: tao::dpi::PhysicalSize<u32>,
    ) {
        let size = size.to_logical::<f32>(window.scale_factor());
        let webviews = self.webviews_lock();

        for webview in webviews.values() {
            let bounds = webview
                .bounds
                .lock()
                .expect("poisoned webview bounds")
                .clone();

            let Some(bounds) = bounds else {
                continue;
            };

            if let Err(error) = webview.set_bounds(wry::Rect {
                position: LogicalPosition::new(
                    size.width * bounds.x_rate,
                    size.height * bounds.y_rate,
                )
                .into(),
                size: LogicalSize::new(
                    size.width * bounds.width_rate,
                    size.height * bounds.height_rate,
                )
                .into(),
            }) {
                eprintln!("failed to autoresize webview: {error}");
            }
        }
    }

    pub fn resize_webviews_with_layout(
        &self,
        window: &Window,
        layout: &FixedLayout,
    ) {
        let size = window.inner_size().to_logical::<f32>(window.scale_factor());

        let webviews = self.webviews_lock();

        let bounds =
            layout.resolve(webviews.values().len(), size.width, size.height);

        for (webview, bounds) in webviews.values().zip(bounds) {
            let _ = webview.set_bounds(bounds.to_wry_rect());
        }
    }
    pub fn create_webview(
        &mut self,
        window: &Window,
        pending: PendingWebview,
    ) -> crate::Result<()> {
        let pending = self.prepare_webview(pending)?;

        let webview = create_wry_webview(
            self.window_label.to_string(),
            window,
            pending,
            self,
        )?;

        self.webviews_lock().insert(webview.label.clone(), webview);

        Ok(())
    }
}
