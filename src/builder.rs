use crate::layout::LayoutBounds;
use crate::{
    attributes::WebviewAttributes,
    pending::PendingWebview,
    types::{ScrollBarStyle, WebviewUrl},
    wrapper::Rect,
};
use dpi::{LogicalPosition, LogicalSize};
use std::path::PathBuf;
use url::Url;

pub struct WebViewBuilder {
    label: String,
    kind: bool,
    attrs: WebviewAttributes,
}

impl WebViewBuilder {
    pub fn new<L>(label: L, url: WebviewUrl) -> Self
    where
        L: Into<String>,
    {
        Self {
            label: label.into(),
            kind: true,
            attrs: WebviewAttributes::new(url),
        }
    }

    pub fn app<L, P>(label: L, path: P) -> Self
    where
        L: Into<String>,
        P: Into<PathBuf>,
    {
        Self::new(label, WebviewUrl::App(path.into()))
    }

    pub fn external<L>(label: L, url: &str) -> crate::Result<Self>
    where
        L: Into<String>,
    {
        let url = Url::parse(url).map_err(crate::Error::InvalidUrl)?;
        Ok(Self::new(label, WebviewUrl::External(url)))
    }

    pub fn custom_protocol<L>(label: L, url: &str) -> crate::Result<Self>
    where
        L: Into<String>,
    {
        let url = Url::parse(url).map_err(crate::Error::InvalidUrl)?;
        Ok(Self::new(label, WebviewUrl::CustomProtocol(url)))
    }

    pub fn kind(mut self, kind: bool) -> Self {
        self.kind = kind;
        self
    }

    pub fn bounds_rect(mut self, bounds: LayoutBounds) -> Self {
        self.attrs.bounds = Some(Rect {
            position: LogicalPosition::new(bounds.x, bounds.y).into(),
            size: LogicalSize::new(bounds.width, bounds.height).into(),
        });

        self
    }

    pub fn auto_resize(mut self) -> Self {
        self.attrs = self.attrs.auto_resize();
        self
    }

    pub fn scroll_bar_style(mut self, style: ScrollBarStyle) -> Self {
        self.attrs = self.attrs.scroll_bar_style(style);
        self
    }

    pub fn devtools(mut self, enabled: bool) -> Self {
        self.attrs = self.attrs.devtools(Some(enabled));
        self
    }

    pub fn user_agent(mut self, user_agent: &str) -> Self {
        self.attrs = self.attrs.user_agent(user_agent);
        self
    }

    pub fn initialization_script(mut self, script: impl Into<String>) -> Self {
        self.attrs = self.attrs.initialization_script(script);
        self
    }

    pub fn build(self) -> PendingWebview {
        PendingWebview::new(self.label, self.kind, self.attrs)
    }
}
