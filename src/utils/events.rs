use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use url::Url;

/// Kind of event for the page load handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLoadEvent {
    /// Page started to load.
    Started,
    /// Page finished loading.
    Finished,
}

/// Download event.
pub enum DownloadEvent<'a> {
    /// Download requested.
    Requested {
        /// The url being downloaded.
        url: Url,
        /// Represents where the file will be downloaded to.
        /// Can be used to set the download location by assigning a new path to it.
        /// The assigned path _must_ be absolute.
        destination: &'a mut PathBuf,
    },
    /// Download finished.
    Finished {
        /// The URL of the original download request.
        url: Url,
        /// Potentially representing the filesystem path the file was downloaded to.
        path: Option<PathBuf>,
        /// Indicates if the download succeeded or not.
        success: bool,
    },
}

/// An event from a window.
#[derive(Debug, Clone)]
pub enum WebviewEvent {
    /// An event associated with the drag and drop action.
    DragDrop(DragDropEvent),
}

/// The drag drop event payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[non_exhaustive]
pub enum DragDropEvent {
    /// A drag operation has entered the webview.
    Enter {
        /// List of paths that are being dragged onto the webview.
        paths: Vec<PathBuf>,
        /// The position of the mouse cursor.
        position: dpi::PhysicalPosition<f64>,
    },
    /// A drag operation is moving over the webview.
    Over {
        /// The position of the mouse cursor.
        position: dpi::PhysicalPosition<f64>,
    },
    /// The file(s) have been dropped onto the webview.
    Drop {
        /// List of paths that are being dropped onto the window.
        paths: Vec<PathBuf>,
        /// The position of the mouse cursor.
        position: dpi::PhysicalPosition<f64>,
    },
    /// The drag operation has been cancelled or left the window.
    Leave,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "target", content = "event", rename_all = "camelCase")]
pub enum SynthesizedEvent {
    Window(SynthesizedWindowEvent),
    WebView(SynthesizedWebViewEvent),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum SynthesizedWindowEvent {
    FocusChanged(bool),
    DragDrop(DragDropEvent),
    FullscreenChanged(bool),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum SynthesizedWebViewEvent {
    DragDrop(DragDropEvent),
}

impl SynthesizedEvent {
    pub fn window_focus_changed(focused: bool) -> Self {
        Self::Window(SynthesizedWindowEvent::FocusChanged(focused))
    }

    pub fn window_drag_drop(event: DragDropEvent) -> Self {
        Self::Window(SynthesizedWindowEvent::DragDrop(event))
    }

    pub fn webview_drag_drop(event: DragDropEvent) -> Self {
        Self::WebView(SynthesizedWebViewEvent::DragDrop(event))
    }

    pub fn window_fullscreen_changed(fullscreen: bool) -> Self {
        Self::Window(SynthesizedWindowEvent::FullscreenChanged(fullscreen))
    }

    pub fn is_window_event(&self) -> bool {
        matches!(self, Self::Window(_))
    }

    pub fn is_webview_event(&self) -> bool {
        matches!(self, Self::WebView(_))
    }
}
