#[cfg(target_os = "macos")]
use crate::utils::types::TitleBarStyle;
use crate::{
    utils::types::{Color, Icon, Theme, WindowSizeConstraints},
    window::{Window, factory::create_window, wrapper::TaoIcon},
};
#[cfg(target_os = "macos")]
use dpi::Position;
use dpi::{PhysicalSize, Size};
use std::{
    fmt,
    sync::{Arc, Mutex},
};
#[cfg(target_os = "android")]
use tao::platform::android::WindowBuilderExtAndroid;
#[cfg(target_os = "ios")]
use tao::platform::ios::WindowBuilderExtIOS;
#[cfg(target_os = "macos")]
use tao::platform::macos::WindowBuilderExtMacOS;
#[cfg(any(
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use tao::platform::unix::WindowBuilderExtUnix;
#[cfg(windows)]
use tao::platform::windows::WindowBuilderExtWindows;
use tao::{
    dpi::{
        LogicalPosition as TaoLogicalPosition, LogicalSize as TaoLogicalSize,
    },
    event_loop::EventLoopWindowTarget as TaoEventLoopWindowTarget,
    window::{
        Fullscreen, Theme as TaoTheme, WindowBuilder as TaoWindowBuilder,
    },
};
#[cfg(windows)]
use windows::Win32::Foundation::HWND;

use crate::utils::error::Error;
/// Builder for configuring and creating a single application window.
///
/// `WindowBuilder` wraps Tao's [`TaoWindowBuilder`] and exposes a stable,
/// application-facing API for common window options. Platform-specific options
/// are guarded with `#[cfg(...)]` so unsupported methods are not compiled for
/// the wrong target.
#[derive(Clone, Default)]
pub struct WindowBuilder {
    pub label: String,
    pub center: bool,
    pub inner: TaoWindowBuilder,
    pub prevent_overflow: Option<Size>,
    #[cfg(windows)]
    pub background_color: Option<tao::window::RGBA>,
    #[cfg(windows)]
    pub is_window_transparent: bool,
    #[cfg(target_os = "macos")]
    pub tabbing_identifier: Option<String>,
}

impl fmt::Debug for WindowBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("WindowBuilder");

        debug
            .field("center", &self.center)
            .field("inner", &self.inner)
            .field("label", &self.label)
            .field("prevent_overflow", &self.prevent_overflow);

        #[cfg(windows)]
        {
            debug
                .field("background_color", &self.background_color)
                .field("is_window_transparent", &self.is_window_transparent);
        }

        #[cfg(target_os = "macos")]
        {
            debug.field("tabbing_identifier", &self.tabbing_identifier);
        }

        debug.finish()
    }
}

impl WindowBuilder {
    /// Creates a new window builder with application defaults.
    ///
    /// Defaults:
    /// - focused window
    /// - title: `Taurino App`
    /// - label: `main`
    /// - Windows class name: `Taurino Window`
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut builder = Self::default()
            .focused(true)
            .label("main")
            .title("Taurino App");

        #[cfg(target_os = "macos")]
        {
            // Tao/webview workaround: the visible title bar keeps the content
            // view inside the native window bounds when devtools are open.
            builder = builder.title_bar_style(TitleBarStyle::Visible);
        }

        #[cfg(windows)]
        {
            builder = builder.window_classname("Taurino Window");
        }

        builder
    }

    /// Sets the native Android activity name used by this window.
    #[cfg(target_os = "android")]
    pub fn activity_name<S: Into<String>>(mut self, class_name: S) -> Self {
        self.inner = self.inner.with_activity_name(class_name.into());
        self
    }

    /// Enables or disables the always-on-bottom window flag.
    pub fn always_on_bottom(mut self, always_on_bottom: bool) -> Self {
        self.inner = self.inner.with_always_on_bottom(always_on_bottom);
        self
    }

    /// Enables or disables the always-on-top window flag.
    pub fn always_on_top(mut self, always_on_top: bool) -> Self {
        self.inner = self.inner.with_always_on_top(always_on_top);
        self
    }

    /// Sets the window background color.
    pub fn background_color(mut self, color: Color) -> Self {
        #[cfg(windows)]
        {
            let color = color.into();
            self.background_color = Some(color);
            self.inner = self.inner.with_background_color(color);
        }

        #[cfg(not(windows))]
        {
            self.inner = self.inner.with_background_color(color.into());
        }

        self
    }

    /// Builds the configured window for the provided Tao event-loop target.
    ///
    /// The resulting [`Window`] owns the native Tao window and starts with no
    /// focused webview registered.
    pub fn build<T: 'static>(
        self,
        window_target: &TaoEventLoopWindowTarget<T>,
    ) -> crate::Result<Window> {
        Ok(create_window(window_target, self)?)
    }

    /// Centers the window after it has been created.
    pub fn center(mut self) -> Self {
        self.center = true;
        self
    }

    /// Enables or disables the native close button.
    pub fn closable(mut self, closable: bool) -> Self {
        self.inner = self.inner.with_closable(closable);
        self
    }

    /// Enables or disables content protection for the window.
    ///
    /// When enabled, the operating system is asked to prevent other
    /// applications from capturing the window content.
    pub fn content_protected(mut self, protected: bool) -> Self {
        self.inner = self.inner.with_content_protection(protected);
        self
    }

    /// Sets the Android activity name that created this window.
    #[cfg(target_os = "android")]
    pub fn created_by_activity_name<S: Into<String>>(
        mut self,
        class_name: S,
    ) -> Self {
        self.inner =
            self.inner.with_created_by_activity_name(class_name.into());
        self
    }

    /// Enables or disables native window decorations.
    pub fn decorations(mut self, decorations: bool) -> Self {
        self.inner = self.inner.with_decorations(decorations);
        self
    }

    /// Enables or disables drag-and-drop support.
    #[cfg(windows)]
    pub fn drag_and_drop(mut self, enabled: bool) -> Self {
        self.inner = self.inner.with_drag_and_drop(enabled);
        self
    }

    /// Enables or disables window focus on creation.
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.inner = self.inner.with_focusable(focusable);
        self
    }

    /// Requests initial focus for the window.
    pub fn focused(mut self, focused: bool) -> Self {
        self.inner = self.inner.with_focused(focused);
        self
    }

    /// Enables or disables fullscreen mode.
    ///
    /// Fullscreen uses a borderless fullscreen configuration on the current
    /// monitor when enabled.
    pub fn fullscreen(mut self, fullscreen: bool) -> Self {
        self.inner = if fullscreen {
            self.inner
                .with_fullscreen(Some(Fullscreen::Borderless(None)))
        } else {
            self.inner.with_fullscreen(None)
        };

        self
    }

    /// Returns the configured window label.
    pub fn get_label(&self) -> &str {
        &self.label
    }

    /// Returns the configured preferred theme.
    pub fn get_theme(&self) -> Option<Theme> {
        self.inner.window.preferred_theme.map(|theme| match theme {
            TaoTheme::Dark => Theme::Dark,
            _ => Theme::Light,
        })
    }

    /// Returns whether a window icon has been configured.
    pub fn has_icon(&self) -> bool {
        self.inner.window.window_icon.is_some()
    }

    /// Hides the native title text while keeping the title bar controls.
    #[cfg(target_os = "macos")]
    pub fn hidden_title(mut self, hidden: bool) -> Self {
        self.inner = self.inner.with_title_hidden(hidden);
        self
    }

    /// Sets the window icon.
    pub fn icon(mut self, icon: Icon) -> crate::Result<Self> {
        let tao_icon = TaoIcon::try_from(icon)?.0;
        self.inner = self.inner.with_window_icon(Some(tao_icon));
        Ok(self)
    }

    /// Sets the initial inner window size in logical pixels.
    pub fn inner_size(mut self, width: f64, height: f64) -> Self {
        self.inner = self
            .inner
            .with_inner_size(TaoLogicalSize::new(width, height));
        self
    }

    /// Sets all inner-size constraints for the window.
    pub fn inner_size_constraints(
        mut self,
        constraints: WindowSizeConstraints,
    ) -> Self {
        self.inner.window.inner_size_constraints =
            tao::window::WindowSizeConstraints {
                min_width: constraints.min_width,
                min_height: constraints.min_height,
                max_width: constraints.max_width,
                max_height: constraints.max_height,
            };

        self
    }

    /// Sets the application-specific window label.
    pub fn label<S: Into<String>>(mut self, label: S) -> Self {
        self.label = label.into();
        self
    }

    /// Sets the maximum inner window size in logical pixels.
    pub fn max_inner_size(mut self, max_width: f64, max_height: f64) -> Self {
        self.inner = self
            .inner
            .with_max_inner_size(TaoLogicalSize::new(max_width, max_height));
        self
    }

    /// Enables or disables the native maximize button.
    pub fn maximizable(mut self, maximizable: bool) -> Self {
        self.inner = self.inner.with_maximizable(maximizable);
        self
    }

    /// Enables or disables maximized state on creation.
    pub fn maximized(mut self, maximized: bool) -> Self {
        self.inner = self.inner.with_maximized(maximized);
        self
    }

    /// Sets the minimum inner window size in logical pixels.
    pub fn min_inner_size(mut self, min_width: f64, min_height: f64) -> Self {
        self.inner = self
            .inner
            .with_min_inner_size(TaoLogicalSize::new(min_width, min_height));
        self
    }

    /// Enables or disables the native minimize button.
    pub fn minimizable(mut self, minimizable: bool) -> Self {
        self.inner = self.inner.with_minimizable(minimizable);
        self
    }

    /// Sets the Windows owner window.
    #[cfg(windows)]
    pub fn owner(mut self, owner: HWND) -> Self {
        self.inner = self.inner.with_owner_window(owner.0 as _);
        self
    }

    /// Sets the Windows parent window.
    #[cfg(windows)]
    pub fn parent(mut self, parent: HWND) -> Self {
        self.inner = self.inner.with_parent_window(parent.0 as _);
        self
    }

    /// Sets the macOS parent window pointer.
    #[cfg(target_os = "macos")]
    pub fn parent(mut self, parent: *mut std::ffi::c_void) -> Self {
        self.inner = self.inner.with_parent_window(parent);
        self
    }

    /// Sets the initial outer window position in logical pixels.
    pub fn position(mut self, x: f64, y: f64) -> Self {
        self.inner = self.inner.with_position(TaoLogicalPosition::new(x, y));
        self
    }

    /// Prevents the initial window bounds from overflowing the working area.
    ///
    /// The working area is the usable monitor area excluding system UI such as
    /// taskbars, docks, or panels.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android:** Unsupported.
    pub fn prevent_overflow(mut self) -> Self {
        self.prevent_overflow
            .replace(PhysicalSize::new(0, 0).into());
        self
    }

    /// Prevents the initial window bounds from overflowing the working area
    /// while preserving the given margin.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android:** Unsupported.
    pub fn prevent_overflow_with_margin(mut self, margin: Size) -> Self {
        self.prevent_overflow.replace(margin);
        self
    }

    /// Requests an iOS scene identifier for this window.
    #[cfg(target_os = "ios")]
    pub fn requested_by_scene_identifier<S: Into<String>>(
        mut self,
        identifier: S,
    ) -> Self {
        self.inner = self
            .inner
            .with_requesting_scene_identifier(identifier.into());
        self
    }

    /// Enables or disables resize support.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.inner = self.inner.with_resizable(resizable);
        self
    }

    /// Enables or disables native window shadow.
    ///
    /// ## Platform-specific
    ///
    /// - **Windows:** Applies undecorated shadow support.
    /// - **macOS:** Applies native window shadow support.
    /// - **Other platforms:** No-op.
    pub fn shadow(#[allow(unused_mut)] mut self, _enable: bool) -> Self {
        #[cfg(windows)]
        {
            self.inner = self.inner.with_undecorated_shadow(_enable);
        }

        #[cfg(target_os = "macos")]
        {
            self.inner = self.inner.with_has_shadow(_enable);
        }

        self
    }

    /// Shows or hides the window icon in the taskbar or window list.
    #[cfg(any(
        windows,
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "linux",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    pub fn skip_taskbar(mut self, skip: bool) -> Self {
        self.inner = self.inner.with_skip_taskbar(skip);
        self
    }

    /// No-op implementation for platforms without taskbar support.
    #[cfg(any(target_os = "android", target_os = "ios", target_os = "macos"))]
    pub fn skip_taskbar(self, _skip: bool) -> Self {
        self
    }

    /// Sets a macOS tabbing identifier used to group compatible windows.
    #[cfg(target_os = "macos")]
    pub fn tabbing_identifier(mut self, identifier: &str) -> Self {
        self.inner = self.inner.with_tabbing_identifier(identifier);
        self.tabbing_identifier.replace(identifier.into());
        self
    }

    /// Sets the preferred window theme.
    pub fn theme(mut self, theme: Option<Theme>) -> Self {
        self.inner = self.inner.with_theme(theme.map(|theme| match theme {
            Theme::Dark => TaoTheme::Dark,
            _ => TaoTheme::Light,
        }));

        self
    }

    /// Sets the initial window title.
    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.inner = self.inner.with_title(title.into());
        self
    }

    /// Sets the macOS title bar style.
    #[cfg(target_os = "macos")]
    pub fn title_bar_style(mut self, style: TitleBarStyle) -> Self {
        match style {
            TitleBarStyle::Visible => {
                self.inner = self.inner.with_titlebar_transparent(false);
                self.inner = self.inner.with_fullsize_content_view(true);
            }
            TitleBarStyle::Transparent => {
                self.inner = self.inner.with_titlebar_transparent(true);
                self.inner = self.inner.with_fullsize_content_view(false);
            }
            TitleBarStyle::Overlay => {
                self.inner = self.inner.with_titlebar_transparent(true);
                self.inner = self.inner.with_fullsize_content_view(true);
            }
            #[allow(unreachable_patterns)]
            unknown => {
                #[cfg(feature = "tracing")]
                tracing::warn!("unknown title bar style applied: {unknown:?}");

                #[cfg(not(feature = "tracing"))]
                eprintln!("unknown title bar style applied: {unknown:?}");
            }
        }

        self
    }

    /// Sets the macOS traffic-light control inset.
    ///
    /// Requires an overlay title bar style and enabled decorations.
    #[cfg(target_os = "macos")]
    pub fn traffic_light_position<P: Into<Position>>(
        mut self,
        position: P,
    ) -> Self {
        self.inner = self.inner.with_traffic_light_inset(position.into());
        self
    }

    /// Enables or disables transparent window background support.
    #[cfg(any(not(target_os = "macos"), feature = "macos-private-api"))]
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.inner = self.inner.with_transparent(transparent);

        #[cfg(windows)]
        {
            self.is_window_transparent = transparent;
        }

        self
    }

    /// Sets the Unix parent window for transient behavior.
    #[cfg(any(
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "linux",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    pub fn transient_for(
        mut self,
        parent: &impl gtk::glib::IsA<gtk::Window>,
    ) -> Self {
        self.inner = self.inner.with_transient_for(parent);
        self
    }

    /// Enables or disables initial visibility.
    pub fn visible(mut self, visible: bool) -> Self {
        self.inner = self.inner.with_visible(visible);
        self
    }

    /// Shows the window on all workspaces where supported by the platform.
    pub fn visible_on_all_workspaces(
        mut self,
        visible_on_all_workspaces: bool,
    ) -> Self {
        self.inner = self
            .inner
            .with_visible_on_all_workspaces(visible_on_all_workspaces);
        self
    }

    /// Sets the Windows window class name.
    #[cfg(windows)]
    pub fn window_classname<S: Into<String>>(
        mut self,
        window_classname: S,
    ) -> Self {
        self.inner = self.inner.with_window_classname(window_classname);
        self
    }

    /// No-op implementation for non-Windows targets.
    #[cfg(not(windows))]
    pub fn window_classname<S: Into<String>>(
        self,
        _window_classname: S,
    ) -> Self {
        self
    }
}

// SAFETY: the builder only stores configuration values before the native window
// is created. It does not share an initialized native window across threads.
#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for WindowBuilder {}
