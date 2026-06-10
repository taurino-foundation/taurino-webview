use std::{
    ops::Deref,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};

use wry::WebView as WryWebView;

use crate::utils::{WebContextStore, WebviewBounds};

pub type WebviewId = u32;

#[derive(Clone)]
pub struct WebView {
    pub label: String,
    pub id: WebviewId,
    pub inner: Rc<WryWebView>,
    pub context_store: WebContextStore,
    /* pub webview_event_listeners: WebviewEventListeners, */
    // the key of the WebContext if it's not shared
    pub context_key: Option<PathBuf>,
    pub(crate) bounds: Arc<Mutex<Option<WebviewBounds>>>,
}

impl Deref for WebView {
    type Target = WryWebView;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Drop for WebView {
    fn drop(&mut self) {
        if Rc::get_mut(&mut self.inner).is_some() {
            let mut context_store = self.context_store.lock().unwrap();

            if let Some(web_context) = context_store.get_mut(&self.context_key)
            {
                web_context.referenced_by_webviews.remove(&self.label);

                // https://github.com/tauri-apps/tauri/issues/14626
                // Because WebKit does not close its network process even when no webviews are running,
                // we need to ensure to re-use the existing process on Linux by keeping the WebContext
                // alive for the lifetime of the app.
                // WebKit on macOS handles this itself.
                #[cfg(not(any(
                    target_os = "linux",
                    target_os = "dragonfly",
                    target_os = "freebsd",
                    target_os = "netbsd",
                    target_os = "openbsd"
                )))]
                if web_context.referenced_by_webviews.is_empty() {
                    context_store.remove(&self.context_key);
                }
            }
        }
    }
}
