use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use crate::attributes::WebviewAttributes;
use crate::utils::{
    DocumentTitleChangedHandler, DownloadHandler, NavigationHandler,
    NewWindowHandler, OnPageLoadHandler, ProxyHandler,
    UriSchemeProtocolHandler, WebResourceRequestHandler, WebviewIpcHandler,
};

/// A webview that has yet to be built.
pub struct PendingWebview {
    /// The label that the webview will be named.
    pub label: String,
    pub kind: bool,
    /// The [`WebviewAttributes`] that the webview will be created with.
    pub webview_attributes: WebviewAttributes,
    pub(crate) uri_scheme_protocols:
        HashMap<String, Box<UriSchemeProtocolHandler>>,
    pub url: String,
    /// How to handle IPC calls on the webview.
    pub ipc_handler: Option<WebviewIpcHandler>,

    /// A handler to decide if incoming url is allowed to navigate.
    pub navigation_handler: Option<Box<NavigationHandler>>,

    pub new_window_handler: Option<Box<NewWindowHandler>>,

    pub document_title_changed_handler:
        Option<Box<DocumentTitleChangedHandler>>,

    #[cfg(target_os = "android")]
    #[allow(clippy::type_complexity)]
    pub on_webview_created: Option<
        Box<
            dyn Fn(CreationContext<'_, '_>) -> Result<(), jni::errors::Error>
                + Send
                + Sync,
        >,
    >,

    pub web_resource_request_handler: Option<Box<WebResourceRequestHandler>>,

    pub on_page_load_handler: Option<Box<OnPageLoadHandler>>,

    pub download_handler: Option<Arc<DownloadHandler>>,

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub on_web_content_process_terminate_handler:
        Option<Box<OnWebContentProcessTerminateHandler>>,

    pub proxy_handler: Option<ProxyHandler>,
}

impl PendingWebview {
    pub fn new<L>(
        label: L,
        kind: bool,
        webview_attributes: WebviewAttributes,
    ) -> Self
    where
        L: Into<String>,
    {
        Self {
            label: label.into(),
            kind,
            webview_attributes,

            url: "taurino://localhost".into(),
            uri_scheme_protocols: HashMap::new(),
            ipc_handler: None,
            navigation_handler: None,
            new_window_handler: None,
            document_title_changed_handler: None,

            #[cfg(target_os = "android")]
            on_webview_created: None,

            web_resource_request_handler: None,
            on_page_load_handler: None,
            download_handler: None,

            #[cfg(any(target_os = "macos", target_os = "ios"))]
            on_web_content_process_terminate_handler: None,

            proxy_handler: None,
        }
    }
    pub(crate) fn internal_register_uri_scheme_protocol<
        N: Into<String>,
        H: Fn(
                &str,
                http::Request<Vec<u8>>,
                Box<dyn FnOnce(http::Response<Cow<'static, [u8]>>) + Send>,
            ) + Send
            + Sync
            + 'static,
    >(
        &mut self,
        uri_scheme: N,
        protocol_handler: H,
    ) {
        let uri_scheme = uri_scheme.into();

        self.uri_scheme_protocols
            .insert(uri_scheme, Box::new(protocol_handler));
    }
}
