pub mod builder;
pub mod factory;
pub mod utils;
pub mod wrapper;

use crate::utils::types::{Color, Theme};
use crate::{
    Result,
    utils::{
        error::Error,
        to_tao_theme,
        types::{
            CursorIcon, Icon, Monitor, ProgressBarState, ResizeDirection,
            WindowSizeConstraints,
        },
    },
    window::wrapper::{
        CursorIconWrapper, MonitorHandleWrapper, PhysicalPositionWrapper,
        PhysicalSizeWrapper, PositionWrapper, ProgressBarStateWrapper,
        SizeWrapper, TaoIcon, UserAttentionTypeWrapper, map_theme,
    },
};
use dpi::{PhysicalPosition, PhysicalSize, Position, Size};
use raw_window_handle::{HandleError, WindowHandle};
use std::{
    fmt,
    sync::{Arc, Mutex},
};
use tao::{
    rwh_06::HasWindowHandle,
    window::{Fullscreen, Window as TaoWindow},
};

#[cfg(target_os = "android")]
use tao::platform::android::WindowExtAndroid;

#[cfg(target_os = "ios")]
use tao::platform::ios::WindowExtIOS;

#[cfg(target_os = "macos")]
use tao::platform::macos::WindowExtMacOS;

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use tao::platform::unix::WindowExtUnix;

#[cfg(windows)]
use tao::platform::windows::WindowExtWindows;

#[cfg(target_os = "macos")]
use crate::utils::types::TitleBarStyle;

/// Runtime wrapper around a Tao window.
///
/// The wrapper keeps the public application label, the platform-specific Tao
/// window handle and the currently focused webview state in one place.
pub struct Window {
    /// Application-specific identifier used to reference this window.
    pub label: String,

    /// Native Tao window instance.
    pub inner: TaoWindow,

    /// Windows-only background color cache.
    #[cfg(windows)]
    pub background_color: Option<tao::window::RGBA>,

    /// Windows-only transparency state cache.
    #[cfg(windows)]
    pub is_window_transparent: bool,

    /// Label of the currently focused webview, if any.
    pub focused_webview: Arc<Mutex<Option<String>>>,
}

impl fmt::Debug for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Window")
            .field("label", &self.label)
            .field("inner", &self.inner)
            .field("focused_webview", &self.focused_webview)
            .finish()
    }
}

impl HasWindowHandle for Window {
    /// Returns the raw window handle exposed by the underlying Tao window.
    fn window_handle(
        &self,
    ) -> std::result::Result<WindowHandle<'_>, HandleError> {
        self.inner.window_handle()
    }
}

impl Window {
    /// Returns the label of the currently focused webview, if one is available.
    ///
    /// Returns `None` when no webview is focused or when the focus state
    /// cannot be read safely.
    pub fn get_focused_webview(&self) -> Option<String> {
        self.focused_webview
            .lock()
            .ok()
            .and_then(|focused_webview| focused_webview.clone())
    }

    /// Returns a shared reference to the underlying Tao window.
    pub fn get_inner(&self) -> &TaoWindow {
        &self.inner
    }

    /// Updates the label of the currently focused webview.
    ///
    /// Pass `Some(label)` to store the focused webview label, or `None` to
    /// clear the current focus state. If the internal mutex is poisoned, the
    /// value is left unchanged and the method still returns successfully.
    pub fn set_focused_webview(
        &self,
        focused_webview: Option<String>,
    ) -> Result<()> {
        if let Ok(mut current_focused_webview) = self.focused_webview.lock() {
            *current_focused_webview = focused_webview;
        }

        Ok(())
    }
    // ---------------------------------------------------------------------
    // Accessors
    // ---------------------------------------------------------------------

    /// Returns the Android activity name associated with this window.
    #[cfg(target_os = "android")]
    pub fn activity_name(&self) -> Result<String> {
        Ok(self.inner.activity_name())
    }

    /// Returns all monitors available to the application.
    pub fn available_monitors(&self) -> Result<Vec<Monitor>> {
        Ok(self
            .inner
            .available_monitors()
            .map(|monitor| MonitorHandleWrapper(monitor).into())
            .collect())
    }

    /// Returns the monitor on which the window currently resides.
    ///
    /// Returns `None` when the current monitor cannot be detected.
    pub fn current_monitor(&self) -> Result<Option<Monitor>> {
        Ok(self
            .inner
            .current_monitor()
            .map(|monitor| MonitorHandleWrapper(monitor).into()))
    }

    /// Returns the default vertical GTK box used by this window.
    ///
    /// ## Platform-specific
    ///
    /// - **Linux / BSD:** Supported through Tao's Unix platform extension.
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    pub fn default_vbox(&self) -> Result<gtk::Box> {
        Ok(self
            .inner
            .default_vbox()
            .expect("Tao did not provide a default GTK vbox for this window")
            .clone())
    }

    /// Returns the GTK application window used by this Tao window.
    ///
    /// ## Platform-specific
    ///
    /// - **Linux / BSD:** Supported through Tao's Unix platform extension.
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    pub fn gtk_window(&self) -> Result<gtk::ApplicationWindow> {
        Ok(self.inner.gtk_window().clone())
    }

    /// Returns the position of the upper-left corner of the window client area.
    ///
    /// The returned position is relative to the upper-left corner of the desktop.
    pub fn inner_position(&self) -> Result<PhysicalPosition<i32>> {
        Ok(self
            .inner
            .inner_position()
            .map(PhysicalPositionWrapper)
            .map(Into::into)
            .map_err(Error::NotSupportedError)?)
    }

    /// Returns whether the window is configured to stay above other windows.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android:** Unsupported.
    pub fn is_always_on_top(&self) -> Result<bool> {
        Ok(self.inner.is_always_on_top())
    }

    /// Returns whether the native close button is enabled.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android:** Unsupported.
    pub fn is_closable(&self) -> Result<bool> {
        Ok(self.inner.is_closable())
    }

    /// Returns whether window decorations are enabled.
    pub fn is_decorated(&self) -> Result<bool> {
        Ok(self.inner.is_decorated())
    }

    /// Returns whether the window currently has focus.
    pub fn is_focused(&self) -> Result<bool> {
        Ok(self.inner.is_focused())
    }

    /// Returns whether the window is currently fullscreen.
    pub fn is_fullscreen(&self) -> Result<bool> {
        Ok(self.inner.fullscreen().is_some())
    }

    /// Returns whether the native maximize button is enabled.
    ///
    /// ## Platform-specific
    ///
    /// - **Linux / iOS / Android:** Unsupported.
    pub fn is_maximizable(&self) -> Result<bool> {
        Ok(self.inner.is_maximizable())
    }

    /// Returns whether the window is currently maximized.
    pub fn is_maximized(&self) -> Result<bool> {
        Ok(self.inner.is_maximized())
    }

    /// Returns whether the native minimize button is enabled.
    ///
    /// ## Platform-specific
    ///
    /// - **Linux / iOS / Android:** Unsupported.
    pub fn is_minimizable(&self) -> Result<bool> {
        Ok(self.inner.is_minimizable())
    }

    /// Returns whether the window is currently minimized.
    pub fn is_minimized(&self) -> Result<bool> {
        Ok(self.inner.is_minimized())
    }

    /// Returns whether the window can be resized by the user.
    pub fn is_resizable(&self) -> Result<bool> {
        Ok(self.inner.is_resizable())
    }

    /// Returns whether the window is currently visible.
    pub fn is_visible(&self) -> Result<bool> {
        Ok(self.inner.is_visible())
    }

    /// Returns the application-specific window label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the monitor that contains the provided physical point.
    pub fn monitor_from_point(
        &self,
        x: f64,
        y: f64,
    ) -> Result<Option<Monitor>> {
        Ok(self
            .inner
            .monitor_from_point(x, y)
            .map(|monitor| MonitorHandleWrapper(monitor).into()))
    }

    /// Returns the position of the upper-left corner of the complete window.
    ///
    /// The returned position includes decorations and is relative to the
    /// upper-left corner of the desktop.
    pub fn outer_position(&self) -> Result<PhysicalPosition<i32>> {
        Ok(self
            .inner
            .outer_position()
            .map(PhysicalPositionWrapper)
            .map(Into::into)
            .map_err(Error::NotSupportedError)?)
    }

    /// Returns the physical size of the complete window.
    ///
    /// This includes the title bar and borders. For content dimensions, use the
    /// inner size API instead.
    pub fn outer_size(&self) -> Result<PhysicalSize<u32>> {
        Ok(PhysicalSizeWrapper(self.inner.outer_size()).into())
    }

    /// Returns the primary system monitor, if one can be identified.
    pub fn primary_monitor(&self) -> Result<Option<Monitor>> {
        Ok(self
            .inner
            .primary_monitor()
            .map(|monitor| MonitorHandleWrapper(monitor).into()))
    }

    /// Returns the scale factor used to convert between logical and physical pixels.
    pub fn scale_factor(&self) -> Result<f64> {
        Ok(self.inner.scale_factor())
    }

    /// Returns the iOS scene identifier associated with this window.
    #[cfg(target_os = "ios")]
    pub fn scene_identifier(&self) -> Result<String> {
        Ok(self.inner.scene_identifier())
    }

    /// Returns the current window theme.
    pub fn theme(&self) -> Result<Theme> {
        Ok(map_theme(&self.inner.theme()))
    }

    /// Returns the current window title.
    pub fn title(&self) -> Result<String> {
        Ok(self.inner.title())
    }

    // ---------------------------------------------------------------------
    // Mutators
    // ---------------------------------------------------------------------

    /// Hides the window.
    pub fn hide(&self) -> Result<()> {
        Ok(self.inner.set_visible(false))
    }

    /// Maximizes the window.
    pub fn maximize(&self) -> Result<()> {
        Ok(self.inner.set_maximized(true))
    }

    /// Minimizes the window.
    pub fn minimize(&self) -> Result<()> {
        Ok(self.inner.set_minimized(true))
    }

    /// Requests user attention for the window.
    ///
    /// Passing `None` clears the current attention request.
    pub fn request_user_attention(
        &self,
        request_type: Option<UserAttentionTypeWrapper>,
    ) -> Result<()> {
        Ok(self
            .inner
            .request_user_attention(request_type.map(|request| request.0)))
    }

    /// Controls whether the window should stay below other windows.
    pub fn set_always_on_bottom(&self, always_on_bottom: bool) -> Result<()> {
        Ok(self.inner.set_always_on_bottom(always_on_bottom))
    }

    /// Controls whether the window should stay above other windows.
    pub fn set_always_on_top(&self, always_on_top: bool) -> Result<()> {
        Ok(self.inner.set_always_on_top(always_on_top))
    }

    /// Sets the window background color.
    pub fn set_background_color(&self, color: Option<Color>) -> Result<()> {
        Ok(self.inner.set_background_color(color.map(Into::into)))
    }

    /// Sets the application badge count.
    ///
    /// `None` and `Some(0)` both clear the badge.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS:** Values outside `i32` are clamped.
    /// - **Windows:** Unsupported. Use `set_overlay_icon` instead.
    /// - **Android:** Unsupported.
    #[cfg(target_os = "ios")]
    pub fn set_badge_count(
        &self,
        count: Option<i64>,
        _desktop_filename: Option<String>,
    ) -> Result<()> {
        Ok(self.inner.set_badge_count(count.map_or(0, |value| {
            value.clamp(i32::MIN as i64, i32::MAX as i64) as i32
        })))
    }

    /// Sets the macOS taskbar badge label.
    ///
    /// Passing `None` clears the badge label.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** Supported.
    #[cfg(target_os = "macos")]
    pub fn set_badge_label(&self, label: Option<String>) -> Result<()> {
        Ok(self.inner.set_badge_label(label))
    }

    /// Enables or disables the native close button.
    ///
    /// ## Platform-specific
    ///
    /// - **Linux:** The window manager may ignore this request for visible windows.
    /// - **iOS / Android:** Unsupported.
    pub fn set_closable(&self, closable: bool) -> Result<()> {
        Ok(self.inner.set_closable(closable))
    }

    /// Protects the window content from being captured by other applications.
    pub fn set_content_protected(&self, protected: bool) -> Result<()> {
        Ok(self.inner.set_content_protection(protected))
    }

    /// Captures or releases the cursor for this window.
    ///
    /// Cursor grabbing does not guarantee that the cursor is hidden. Use
    /// `set_cursor_visible(false)` when the cursor should also be hidden.
    pub fn set_cursor_grab(&self, grab: bool) -> Result<()> {
        Ok(self
            .inner
            .set_cursor_grab(grab)
            .map_err(Error::ExternalError)?)
    }

    /// Sets the cursor icon used while the cursor is over this window.
    pub fn set_cursor_icon(&self, icon: CursorIcon) -> Result<()> {
        Ok(self.inner.set_cursor_icon(CursorIconWrapper::from(icon).0))
    }

    /// Moves the cursor to the provided window-relative position.
    pub fn set_cursor_position<Pos>(&self, position: Pos) -> Result<()>
    where
        Pos: Into<Position>,
    {
        Ok(self
            .inner
            .set_cursor_position(PositionWrapper::from(position.into()).0)
            .map_err(Error::ExternalError)?)
    }

    /// Shows or hides the cursor while it is over this window.
    pub fn set_cursor_visible(&self, visible: bool) -> Result<()> {
        Ok(self.inner.set_cursor_visible(visible))
    }

    /// Enables or disables native window decorations.
    pub fn set_decorations(&self, decorations: bool) -> Result<()> {
        Ok(self.inner.set_decorations(decorations))
    }

    /// Enables or disables the window.
    ///
    /// ## Platform-specific
    ///
    /// - **Android / iOS:** Unsupported.
    pub fn set_enabled(&self, enabled: bool) -> Result<()> {
        Ok(self.inner.set_enable(enabled))
    }

    /// Brings the window to the front and requests focus.
    pub fn set_focus(&self) -> Result<()> {
        Ok(self.inner.set_focus())
    }

    /// Controls whether this window can receive focus.
    pub fn set_focusable(&self, focusable: bool) -> Result<()> {
        Ok(self.inner.set_focusable(focusable))
    }

    /// Enables or disables borderless fullscreen mode.
    pub fn set_fullscreen(&self, fullscreen: bool) -> Result<()> {
        if fullscreen {
            self.inner
                .set_fullscreen(Some(Fullscreen::Borderless(None)));
        } else {
            self.inner.set_fullscreen(None);
        }

        Ok(())
    }

    /// Sets the window icon.
    pub fn set_icon(&self, icon: Icon<'_>) -> Result<()> {
        self.inner.set_window_icon(Some(TaoIcon::try_from(icon)?.0));
        Ok(())
    }

    /// Enables or disables cursor event passthrough for this window.
    pub fn set_ignore_cursor_events(&self, ignore: bool) -> Result<()> {
        Ok(self
            .inner
            .set_ignore_cursor_events(ignore)
            .map_err(Error::ExternalError)?)
    }

    /// Sets the maximum inner window size.
    pub fn set_max_size(&self, size: Option<Size>) -> Result<()> {
        Ok(self
            .inner
            .set_max_inner_size(size.map(|size| SizeWrapper::from(size).0)))
    }

    /// Enables or disables the native maximize button.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** Controls the zoom button in the title bar.
    /// - **Linux / iOS / Android:** Unsupported.
    pub fn set_maximizable(&self, maximizable: bool) -> Result<()> {
        Ok(self.inner.set_maximizable(maximizable))
    }

    /// Sets the minimum inner window size.
    pub fn set_min_size(&self, size: Option<Size>) -> Result<()> {
        Ok(self
            .inner
            .set_min_inner_size(size.map(|size| SizeWrapper::from(size).0)))
    }

    /// Enables or disables the native minimize button.
    ///
    /// ## Platform-specific
    ///
    /// - **Linux / iOS / Android:** Unsupported.
    pub fn set_minimizable(&self, minimizable: bool) -> Result<()> {
        Ok(self.inner.set_minimizable(minimizable))
    }

    /// Sets or clears the Windows taskbar overlay icon.
    ///
    /// ## Platform-specific
    ///
    /// - **Windows:** Supported.
    #[cfg(windows)]
    pub fn set_overlay_icon(&self, icon: Option<Icon<'_>>) -> Result<()> {
        let tao_icon = icon.map(TaoIcon::try_from).transpose()?;
        self.inner
            .set_overlay_icon(tao_icon.as_ref().map(|icon| &icon.0));
        Ok(())
    }

    /// Sets the outer window position.
    pub fn set_position(&self, position: Position) -> Result<()> {
        Ok(self
            .inner
            .set_outer_position(PositionWrapper::from(position).0))
    }

    /// Sets the taskbar progress state.
    ///
    /// ## Platform-specific
    ///
    /// - **Linux / macOS:** Progress is app-wide and requires a supported desktop environment.
    /// - **iOS / Android:** Unsupported.
    pub fn set_progress_bar(
        &self,
        progress_state: ProgressBarState,
    ) -> Result<()> {
        Ok(self
            .inner
            .set_progress_bar(ProgressBarStateWrapper::from(progress_state).0))
    }

    /// Enables or disables simple fullscreen mode.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** Supported.
    #[cfg(target_os = "macos")]
    pub fn set_simple_fullscreen(&self, enable: bool) -> Result<()> {
        Ok(self.inner.set_simple_fullscreen(enable))
    }

    /// Sets the inner window size.
    pub fn set_size(&self, size: Size) -> Result<()> {
        Ok(self.inner.set_inner_size(SizeWrapper::from(size).0))
    }

    /// Sets the minimum and maximum inner size constraints.
    pub fn set_size_constraints(
        &self,
        constraints: WindowSizeConstraints,
    ) -> Result<()> {
        self.inner.set_inner_size_constraints(
            tao::window::WindowSizeConstraints {
                min_width: constraints.min_width,
                min_height: constraints.min_height,
                max_width: constraints.max_width,
                max_height: constraints.max_height,
            },
        );

        Ok(())
    }

    /// Controls whether the window icon is hidden from the taskbar.
    ///
    /// ## Platform-specific
    ///
    /// - **Windows / Linux / BSD:** Supported.
    #[cfg(any(
        windows,
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    pub fn set_skip_taskbar(&self, skip: bool) -> Result<()> {
        Ok(self
            .inner
            .set_skip_taskbar(skip)
            .map_err(Error::ExternalError)?)
    }

    /// Sets the preferred theme for this window.
    ///
    /// ## Platform-specific
    ///
    /// - **Linux / macOS:** Theme is app-wide and not window-specific.
    /// - **iOS / Android:** Unsupported.
    pub fn set_theme(&self, theme: Option<Theme>) -> Result<()> {
        Ok(self.inner.set_theme(to_tao_theme(theme)))
    }

    /// Updates the window title.
    pub fn set_title<S>(&self, title: S) -> Result<()>
    where
        S: Into<String>,
    {
        let title = title.into();
        Ok(self.inner.set_title(&title))
    }

    /// Sets the macOS title bar style.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** Supported.
    /// - **Linux / Windows / iOS / Android:** Unsupported.
    #[cfg(target_os = "macos")]
    pub fn set_title_bar_style(&self, style: TitleBarStyle) -> Result<()> {
        match style {
            TitleBarStyle::Overlay => {
                self.inner.set_titlebar_transparent(true);
                self.inner.set_fullsize_content_view(true);
            }
            TitleBarStyle::Transparent => {
                self.inner.set_titlebar_transparent(true);
                self.inner.set_fullsize_content_view(false);
            }
            TitleBarStyle::Visible => {
                self.inner.set_titlebar_transparent(false);
                self.inner.set_fullsize_content_view(false);
            }
            #[allow(unreachable_patterns)]
            unknown => {
                eprintln!("unknown title bar style applied: {unknown:?}");
            }
        }

        Ok(())
    }

    /// Sets the macOS traffic-light button position.
    ///
    /// This requires `TitleBarStyle::Overlay` and enabled decorations.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** Supported.
    /// - **Linux / Windows / iOS / Android:** Unsupported.
    #[cfg(target_os = "macos")]
    pub fn set_traffic_light_position(&self, position: Position) -> Result<()> {
        Ok(self.inner.set_traffic_light_inset(position))
    }

    /// Controls whether the window is visible on all workspaces.
    pub fn set_visible_on_all_workspaces(
        &self,
        visible_on_all_workspaces: bool,
    ) -> Result<()> {
        Ok(self
            .inner
            .set_visible_on_all_workspaces(visible_on_all_workspaces))
    }

    /// Shows the window.
    pub fn show(&self) -> Result<()> {
        Ok(self.inner.set_visible(true))
    }

    /// Starts an interactive window drag operation.
    pub fn start_dragging(&self) -> Result<()> {
        Ok(self.inner.drag_window().map_err(Error::ExternalError)?)
    }

    /// Starts an interactive resize operation in the provided direction.
    pub fn start_resize_dragging(
        &self,
        direction: ResizeDirection,
    ) -> Result<()> {
        let direction = match direction {
            ResizeDirection::East => tao::window::ResizeDirection::East,
            ResizeDirection::North => tao::window::ResizeDirection::North,
            ResizeDirection::NorthEast => {
                tao::window::ResizeDirection::NorthEast
            }
            ResizeDirection::NorthWest => {
                tao::window::ResizeDirection::NorthWest
            }
            ResizeDirection::South => tao::window::ResizeDirection::South,
            ResizeDirection::SouthEast => {
                tao::window::ResizeDirection::SouthEast
            }
            ResizeDirection::SouthWest => {
                tao::window::ResizeDirection::SouthWest
            }
            ResizeDirection::West => tao::window::ResizeDirection::West,
        };

        Ok(self
            .inner
            .drag_resize_window(direction)
            .map_err(Error::ExternalError)?)
    }

    /// Restores the window from maximized state.
    pub fn unmaximize(&self) -> Result<()> {
        Ok(self.inner.set_maximized(false))
    }

    /// Restores the window from minimized state.
    pub fn unminimize(&self) -> Result<()> {
        Ok(self.inner.set_minimized(false))
    }
}
