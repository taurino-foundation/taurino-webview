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
    protocol::pattern::{Pattern, PatternJavascript},
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

/// Runtime configuration for the webview manager.
///
/// This configuration controls how webviews are initialized, which frontend
/// source is used, whether isolation is enabled, and which JavaScript runtime
/// bootstrap scripts are injected into each webview.
///
/// The configuration is designed to be Python-friendly:
/// Python can decide whether isolation should be enabled and can provide
/// isolation assets as raw bytes at runtime.
pub struct ManagerConfig {
    /// Enables or disables the isolation runtime.
    ///
    /// This is an explicit runtime option. It decides whether isolation-related
    /// JavaScript and protocols should be installed.
    ///
    /// Important:
    /// `Pattern::Isolation` stores the isolation data, but this flag decides
    /// whether that data is actually used.
    isolation_enabled: bool,

    /// Enables prototype freezing in the injected JavaScript runtime.
    ///
    /// When enabled, the runtime may inject JavaScript that freezes selected
    /// prototypes to reduce prototype-pollution risks.
    freeze_prototype: bool,

    /// The current application pattern.
    ///
    /// `Pattern::Brownfield` represents the normal application mode.
    /// `Pattern::Isolation` contains the runtime isolation assets and metadata.
    ///
    /// The pattern alone does not activate isolation. Isolation is active only
    /// when `isolation_enabled == true` and this field is `Pattern::Isolation`.
    pattern: Arc<Pattern>,

    /// Indicates whether a native webview runtime is available on the system.
    ///
    /// This is detected by checking `wry::webview_version()`.
    webview_runtime_installed: bool,

    /// JavaScript used to initialize the invoke system.
    ///
    /// This script is injected into the webview as part of the runtime bootstrap.
    invoke_initialization_script: String,

    /// Runtime-generated invoke key.
    ///
    /// This key can be used to protect internal invoke calls.
    pub(crate) invoke_key: String,

    /// Defines where the frontend is loaded from.
    ///
    /// This can be a development server URL, a static directory, a list of files,
    /// or `None`, in which case the internal application protocol is used.
    frontend_dist: Option<FrontendDist>,

    /// Shared web context storage.
    ///
    /// This is used to manage persistent webview context data.
    web_context: WebContextStore,
}

impl ManagerConfig {
    /// Creates a new manager configuration using default values.
    pub fn new() -> crate::Result<Self> {
        Ok(Self::default())
    }

    // ---------------------------------------------------------------------
    // State checks
    // ---------------------------------------------------------------------

    /// Returns whether isolation was explicitly enabled in the configuration.
    ///
    /// This only reflects the option value. It does not guarantee that a valid
    /// `Pattern::Isolation` is currently configured.
    pub fn is_isolation_enabled(&self) -> bool {
        self.isolation_enabled
    }

    /// Returns whether isolation is fully active.
    ///
    /// Isolation is active only when:
    ///
    /// - the isolation option is enabled
    /// - the configured pattern is `Pattern::Isolation`
    ///
    /// Use this method before injecting isolation JavaScript or registering the
    /// isolation protocol.
    pub fn is_isolation_active(&self) -> bool {
        self.isolation_enabled
            && matches!(self.pattern.as_ref(), Pattern::Isolation { .. })
    }

    /// Returns whether prototype freezing is enabled.
    pub fn freeze_prototype(&self) -> bool {
        self.freeze_prototype
    }

    /// Returns whether the system webview runtime is installed.
    pub fn webview_runtime_installed(&self) -> bool {
        self.webview_runtime_installed
    }

    // ---------------------------------------------------------------------
    // Accessors
    // ---------------------------------------------------------------------

    /// Returns a cloned reference-counted handle to the current pattern.
    pub fn pattern(&self) -> Arc<Pattern> {
        Arc::clone(&self.pattern)
    }

    /// Returns a borrowed reference to the current pattern.
    ///
    /// Prefer this method when cloning the `Arc` is not required.
    pub fn pattern_ref(&self) -> &Pattern {
        self.pattern.as_ref()
    }

    /// Returns the invoke initialization script.
    pub fn invoke_initialization_script(&self) -> &str {
        &self.invoke_initialization_script
    }

    /// Returns the runtime invoke key.
    pub(crate) fn invoke_key(&self) -> &str {
        &self.invoke_key
    }

    /// Returns the configured frontend distribution source.
    pub fn frontend_dist(&self) -> Option<&FrontendDist> {
        self.frontend_dist.as_ref()
    }

    /// Returns the configured frontend resource path.
    ///
    /// This is an internal alias used by the protocol layer.
    pub(crate) fn resource_path(&self) -> Option<&FrontendDist> {
        self.frontend_dist.as_ref()
    }

    /// Returns the shared web context store.
    pub(crate) fn web_context(&self) -> &WebContextStore {
        &self.web_context
    }

    // ---------------------------------------------------------------------
    // Pattern and isolation configuration
    // ---------------------------------------------------------------------

    /// Enables or disables the isolation option.
    ///
    /// Disabling isolation also resets the pattern to `Pattern::Brownfield`
    /// to avoid stale isolation assets being used accidentally.
    ///
    /// Enabling isolation does not create isolation assets automatically.
    /// Use `set_isolation_html` or `set_isolation_assets` for that.
    #[must_use]
    pub fn set_isolation_enabled(mut self, enabled: bool) -> Self {
        self.isolation_enabled = enabled;

        if !enabled {
            self.pattern = Arc::new(Pattern::Brownfield);
        }

        self
    }

    /// Configures the manager for normal Brownfield mode.
    ///
    /// This disables isolation and resets the pattern to `Pattern::Brownfield`.
    #[must_use]
    pub fn set_brownfield(mut self) -> Self {
        self.isolation_enabled = false;
        self.pattern = Arc::new(Pattern::Brownfield);
        self
    }

    /// Sets the raw application pattern without changing the isolation option.
    ///
    /// This is useful for advanced internal use cases.
    ///
    /// For Python-facing APIs, prefer `set_brownfield`, `set_isolation_html`,
    /// or `set_isolation_assets`, because those methods keep the option state
    /// and the pattern state consistent.
    #[must_use]
    pub fn set_pattern(mut self, pattern: Arc<Pattern>) -> Self {
        self.pattern = pattern;
        self
    }

    /// Enables isolation and creates an isolation pattern from a single HTML file.
    ///
    /// This is the recommended Python-facing entry point for simple isolation
    /// setups where Python provides the isolation page as raw bytes.
    ///
    /// Python example:
    ///
    /// ```python
    /// config.set_isolation_html(b"<html>...</html>")
    /// ```
    #[must_use]
    pub fn set_isolation_html<B>(mut self, html: B) -> Self
    where
        B: Into<Vec<u8>>,
    {
        self.isolation_enabled = true;
        self.pattern = Arc::new(Pattern::isolation_from_html(html));
        self
    }

    /// Enables isolation and creates an isolation pattern from multiple assets.
    ///
    /// The asset map uses normalized web paths as keys and raw file contents
    /// as values.
    ///
    /// Python example:
    ///
    /// ```python
    /// config.set_isolation_assets({
    ///     "index.html": b"<html></html>",
    ///     "main.js": b"console.log('hello')",
    ///     "style.css": b"body { margin: 0 }",
    /// })
    /// ```
    #[must_use]
    pub fn set_isolation_assets(
        mut self,
        assets: HashMap<String, Vec<u8>>,
    ) -> Self {
        self.isolation_enabled = true;
        self.pattern = Arc::new(Pattern::isolation_from_bytes_map(assets));
        self
    }

    // ---------------------------------------------------------------------
    // Runtime configuration
    // ---------------------------------------------------------------------

    /// Enables or disables prototype freezing.
    #[must_use]
    pub fn set_freeze_prototype(mut self, value: bool) -> Self {
        self.freeze_prototype = value;
        self
    }

    /// Overrides whether the webview runtime is considered installed.
    ///
    /// This is mostly useful for tests or custom runtime checks.
    #[must_use]
    pub fn set_webview_runtime_installed(mut self, value: bool) -> Self {
        self.webview_runtime_installed = value;
        self
    }

    /// Sets the JavaScript used to initialize the invoke system.
    #[must_use]
    pub fn set_invoke_initialization_script<S>(mut self, script: S) -> Self
    where
        S: Into<String>,
    {
        self.invoke_initialization_script = script.into();
        self
    }

    /// Sets the runtime invoke key.
    pub(crate) fn set_invoke_key<S>(mut self, key: S) -> Self
    where
        S: Into<String>,
    {
        self.invoke_key = key.into();
        self
    }

    /// Sets the shared web context store.
    #[must_use]
    pub fn set_web_context(mut self, web_context: WebContextStore) -> Self {
        self.web_context = web_context;
        self
    }

    // ---------------------------------------------------------------------
    // Frontend source configuration
    // ---------------------------------------------------------------------

    /// Sets the frontend distribution source.
    #[must_use]
    pub fn set_frontend_dist(mut self, frontend_dist: FrontendDist) -> Self {
        self.frontend_dist = Some(frontend_dist);
        self
    }

    /// Clears the frontend distribution source.
    ///
    /// When no frontend distribution is configured, the manager falls back to
    /// the internal application protocol.
    #[must_use]
    pub fn clear_frontend_dist(mut self) -> Self {
        self.frontend_dist = None;
        self
    }

    /// Configures a development server URL as the frontend source.
    ///
    /// This is useful during development when the frontend is served by tools
    /// such as Vite, Next.js, Vue CLI, or another local web server.
    pub fn with_dev_server_url(mut self, url: &str) -> crate::Result<Self> {
        let url = Url::parse(url).map_err(crate::Error::InvalidUrl)?;
        self.frontend_dist = Some(FrontendDist::Url(url));
        Ok(self)
    }

    /// Configures a static directory as the frontend source.
    #[must_use]
    pub fn set_static_dir<P>(mut self, path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        self.frontend_dist = Some(FrontendDist::Directory(path.into()));
        self
    }

    /// Configures a list of static frontend files as the frontend source.
    #[must_use]
    pub fn set_static_files(mut self, files: Vec<PathBuf>) -> Self {
        self.frontend_dist = Some(FrontendDist::Files(files));
        self
    }
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            isolation_enabled: false,
            freeze_prototype: false,
            pattern: Arc::new(Pattern::Brownfield),
            webview_runtime_installed: wry::webview_version().is_ok(),
            invoke_initialization_script: String::new(),
            invoke_key: String::new(),
            frontend_dist: None,
            web_context: WebContextStore::default(),
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
        // Ensure that each webview label is unique within this manager.
        if self.webviews_lock().contains_key(&pending.label) {
            return Err(crate::Error::WebviewLabelAlreadyExists(pending.label));
        }

        let label = pending.label.clone();
        let use_https_scheme = pending.webview_attributes.use_https_scheme;

        // Resolve the final URL that should be loaded by the webview.
        //
        // App URLs are resolved against the configured frontend source.
        // External URLs are preserved unless they point to the local app server,
        // in which case they are mapped to the internal application protocol.
        let mut url = match &pending.webview_attributes.url {
            WebviewUrl::App(path) => {
                let app_url = self.get_app_url(use_https_scheme);

                let base_url = if is_local_network_url(&app_url) {
                    Cow::Owned(
                        Url::parse("taurino://localhost")
                            .expect("invalid internal application URL"),
                    )
                } else {
                    app_url
                };

                // Keep the base URL clean when the requested path is `index.html`.
                if path.to_str() == Some("index.html") {
                    base_url.into_owned()
                } else {
                    base_url
                        .join(&path.to_string_lossy())
                        .map_err(crate::Error::InvalidUrl)?
                }
            }

            WebviewUrl::External(url) => {
                let app_url = self.get_app_url(use_https_scheme);
                let is_app_url = app_url.make_relative(url).is_some();

                if is_app_url && is_local_network_url(url) {
                    Url::parse("taurino://localhost")
                        .expect("invalid internal application URL")
                } else {
                    url.clone()
                }
            }

            WebviewUrl::CustomProtocol(url) => url.clone(),

            #[allow(unreachable_patterns)]
            _ => unimplemented!(),
        };

        // Normalize inline HTML data URLs.
        //
        // This keeps data URLs usable when the decoded body is HTML.
        // The check is intentionally conservative and only rewrites URLs that
        // look like HTML documents.
        if url.scheme() == "data" {
            if let Ok(data_url) = data_url::DataUrl::process(url.as_str()) {
                if let Ok((body, _)) = data_url.decode_to_vec() {
                    let html = String::from_utf8_lossy(&body).into_owned();

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
        let isolation_active = self.config.is_isolation_active();

        let mut all_initialization_scripts: Vec<InitializationScript> =
            Vec::new();

        fn main_frame_script(script: String) -> InitializationScript {
            InitializationScript {
                script,
                for_main_frame_only: true,
            }
        }

        // ---------------------------------------------------------------------
        // Base runtime bootstrap
        // ---------------------------------------------------------------------

        // This script is always injected. It creates the global runtime namespace
        // used by the framework and plugins.
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

        // Inject metadata about the current window and webview.
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

        // ---------------------------------------------------------------------
        // Optional isolation runtime scripts
        // ---------------------------------------------------------------------

        // Pattern JavaScript is only generated when isolation is actually active.
        //
        // This keeps Brownfield mode clean and avoids injecting unused isolation
        // configuration into normal webviews.
        let pattern_script = if isolation_active {
            PatternJavascript {
                pattern: self.config.pattern_ref().into(),
            }
            .render_default(&Default::default())?
            .into_string()
        } else {
            String::new()
        };

        // IPC JavaScript receives the isolation origin only when isolation is active.
        let ipc_script = if isolation_active {
            let isolation_origin = match self.config.pattern_ref() {
                Pattern::Isolation { schema, .. } => {
                    crate::protocol::pattern::format_real_schema(
                        schema,
                        use_https_scheme,
                    )
                }
                Pattern::Brownfield => String::new(),
            };

            IpcJavascript {
                isolation_origin: &isolation_origin,
            }
            .render_default(&Default::default())?
            .into_string()
        } else {
            String::new()
        };

        // Inject the main runtime initialization script.
        //
        // In Brownfield mode, `pattern_script` and `ipc_script` are empty.
        // In Isolation mode, they contain the required isolation runtime setup.
        all_initialization_scripts.push(main_frame_script(
            self.initialization_script(
                &ipc_script,
                &pattern_script,
                use_https_scheme,
            )?,
        ));

        // Inject the isolation iframe bootstrap only when isolation is active.
        if isolation_active {
            if let Pattern::Isolation { schema, .. } = self.config.pattern_ref()
            {
                let isolation_src =
                    crate::protocol::pattern::format_real_schema(
                        schema,
                        use_https_scheme,
                    );

                all_initialization_scripts.push(main_frame_script(
                IsolationJavascript {
                    isolation_src: &isolation_src,
                    style: "position: fixed; top: 0; left: 0; width: 100%; height: 100%; border: 0;",
                }
                .render_default(&Default::default())?
                .into_string(),
            ));
            }
        }

        // ---------------------------------------------------------------------
        // Prepend framework scripts before user-defined scripts
        // ---------------------------------------------------------------------
        // Prepend `all_initialization_scripts` to `webview_attributes.initialization_scripts`
        all_initialization_scripts
            .extend(pending.webview_attributes.initialization_scripts);
        pending.webview_attributes.initialization_scripts =
            all_initialization_scripts;

        pending.webview_attributes = pending.webview_attributes;
        // ---------------------------------------------------------------------
        // Register custom protocols
        // ---------------------------------------------------------------------

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

        // ---------------------------------------------------------------------
        // Register built-in protocols
        // ---------------------------------------------------------------------

        let window_url =
            Url::parse(&pending.url).map_err(crate::Error::InvalidUrl)?;

        let window_origin = Self::window_origin(&window_url, use_https_scheme);

        self.register_builtin_protocols(
            &mut pending,
            &mut registered_scheme_protocols,
            window_origin,
            use_https_scheme,
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
        use_https_scheme: bool,
    ) {
        if !registered_scheme_protocols.contains(APP_PROTOCOL) {
            let _web_resource_request_handler =
                pending.web_resource_request_handler.take();

            let app_protocol = protocol::taurino::get(
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

        if self.config.is_isolation_active() {
            if let Pattern::Isolation {
                assets,
                schema,
                key: _,
                crypto_keys: _,
            } = self.config.pattern_ref()
            {
                if !registered_scheme_protocols.contains(schema) {
                    let protocol = crate::protocol::isolation::get(
                        schema.clone(),
                        Arc::clone(assets),
                        window_origin.clone(),
                        use_https_scheme,
                    );

                    pending.internal_register_uri_scheme_protocol(
                        schema.clone(),
                        move |webview_id, request, responder| {
                            protocol.handle(
                                webview_id,
                                request,
                                ManagerUriSchemeResponder(responder),
                            );
                        },
                    );

                    registered_scheme_protocols.insert(schema.clone());
                }
            }
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
