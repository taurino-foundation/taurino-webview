use std::{fmt, path::PathBuf, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::skip_serializing_none;
use url::Url;

/// Identifier of a window.
/* #[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct WindowId(u32);

impl From<u32> for WindowId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
 */

/// The scrollbar style to use in the webview.
///
/// ## Platform-specific
///
/// - **Windows**: This option must be given the same value for all webviews that target the same data directory.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default)]
pub enum ScrollBarStyle {
    #[default]
    /// The default scrollbar style for the webview.
    Default,

    #[cfg(windows)]
    /// Fluent UI style overlay scrollbars. **Windows Only**
    ///
    /// Requires WebView2 Runtime version 125.0.2535.41 or higher, does nothing on older versions,
    /// see <https://learn.microsoft.com/en-us/microsoft-edge/webview2/release-notes/?tabs=dotnetcsharp#10253541>
    FluentOverlay,
}

/// Defines the URL or assets to embed in the application.
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged, deny_unknown_fields)]
#[non_exhaustive]
pub enum FrontendDist {
    /// An external URL that should be used as the default application URL. No assets are embedded in the app in this case.
    Url(Url),
    /// Path to a directory containing the frontend dist assets.
    Directory(PathBuf),
    /// An array of files to embed in the app.
    Files(Vec<PathBuf>),
}

impl std::fmt::Display for FrontendDist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Url(url) => write!(f, "{url}"),
            Self::Directory(p) => write!(f, "{}", p.display()),
            Self::Files(files) => {
                write!(f, "{}", serde_json::to_string(files).unwrap())
            }
        }
    }
}

#[allow(deprecated)]
mod window_effects {
    use super::*;

    #[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize)]
    #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
    #[serde(rename_all = "camelCase")]
    /// Platform-specific window effects
    pub enum WindowEffect {
        /// A default material appropriate for the view's effectiveAppearance. **macOS 10.14-**
        #[deprecated(
            since = "macOS 10.14",
            note = "You should instead choose an appropriate semantic material."
        )]
        AppearanceBased,
        /// **macOS 10.14-**
        #[deprecated(since = "macOS 10.14", note = "Use a semantic material instead.")]
        Light,
        /// **macOS 10.14-**
        #[deprecated(since = "macOS 10.14", note = "Use a semantic material instead.")]
        Dark,
        /// **macOS 10.14-**
        #[deprecated(since = "macOS 10.14", note = "Use a semantic material instead.")]
        MediumLight,
        /// **macOS 10.14-**
        #[deprecated(since = "macOS 10.14", note = "Use a semantic material instead.")]
        UltraDark,
        /// **macOS 10.10+**
        Titlebar,
        /// **macOS 10.10+**
        Selection,
        /// **macOS 10.11+**
        Menu,
        /// **macOS 10.11+**
        Popover,
        /// **macOS 10.11+**
        Sidebar,
        /// **macOS 10.14+**
        HeaderView,
        /// **macOS 10.14+**
        Sheet,
        /// **macOS 10.14+**
        WindowBackground,
        /// **macOS 10.14+**
        HudWindow,
        /// **macOS 10.14+**
        FullScreenUI,
        /// **macOS 10.14+**
        Tooltip,
        /// **macOS 10.14+**
        ContentBackground,
        /// **macOS 10.14+**
        UnderWindowBackground,
        /// **macOS 10.14+**
        UnderPageBackground,
        /// Mica effect that matches the system dark preference **Windows 11 Only**
        Mica,
        /// Mica effect with dark mode but only if dark mode is enabled on the system **Windows 11 Only**
        MicaDark,
        /// Mica effect with light mode **Windows 11 Only**
        MicaLight,
        /// Tabbed effect that matches the system dark preference **Windows 11 Only**
        Tabbed,
        /// Tabbed effect with dark mode but only if dark mode is enabled on the system **Windows 11 Only**
        TabbedDark,
        /// Tabbed effect with light mode **Windows 11 Only**
        TabbedLight,
        /// **Windows 7/10/11(22H1) Only**
        ///
        /// ## Notes
        ///
        /// This effect has bad performance when resizing/dragging the window on Windows 11 build 22621.
        Blur,
        /// **Windows 10/11 Only**
        ///
        /// ## Notes
        ///
        /// This effect has bad performance when resizing/dragging the window on Windows 10 v1903+ and Windows 11 build 22000.
        Acrylic,
    }

    /// Window effect state **macOS only**
    ///
    /// <https://developer.apple.com/documentation/appkit/nsvisualeffectview/state>
    #[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize)]
    #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
    #[serde(rename_all = "camelCase")]
    pub enum WindowEffectState {
        /// Make window effect state follow the window's active state
        FollowsWindowActiveState,
        /// Make window effect state always active
        Active,
        /// Make window effect state always inactive
        Inactive,
    }
}

pub use window_effects::{WindowEffect, WindowEffectState};

/// How the window title bar should be displayed on macOS.
#[derive(Debug, Clone, PartialEq, Eq, Copy, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum TitleBarStyle {
    /// A normal title bar.
    #[default]
    Visible,
    /// Makes the title bar transparent, so the window background color is shown instead.
    ///
    /// Useful if you don't need to have actual HTML under the title bar. This lets you avoid the caveats of using `TitleBarStyle::Overlay`. Will be more useful when Tauri lets you set a custom window background color.
    Transparent,
    /// Shows the title bar as a transparent overlay over the window's content.
    ///
    /// Keep in mind:
    /// - The height of the title bar is different on different OS versions, which can lead to window the controls and title not being where you don't expect.
    /// - You need to define a custom drag region to make your window draggable, however due to a limitation you can't drag the window when it's not in focus <https://github.com/tauri-apps/tauri/issues/4316>.
    /// - The color of the window title depends on the system theme.
    Overlay,
}

impl Serialize for TitleBarStyle {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

impl<'de> Deserialize<'de> for TitleBarStyle {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_lowercase().as_str() {
            "transparent" => Self::Transparent,
            "overlay" => Self::Overlay,
            _ => Self::Visible,
        })
    }
}

impl std::fmt::Display for TitleBarStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Visible => "Visible",
                Self::Transparent => "Transparent",
                Self::Overlay => "Overlay",
            }
        )
    }
}

/// System theme.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum Theme {
    /// Light theme.
    Light,
    /// Dark theme.
    Dark,
}

impl Serialize for Theme {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

impl<'de> Deserialize<'de> for Theme {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_lowercase().as_str() {
            "dark" => Self::Dark,
            _ => Self::Light,
        })
    }
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Light => "light",
                Self::Dark => "dark",
            }
        )
    }
}

/// A tuple struct of RGBA colors. Each value has minimum of 0 and maximum of 255.
#[derive(Debug, PartialEq, Eq, Serialize, Default, Clone, Copy)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl From<Color> for (u8, u8, u8, u8) {
    fn from(value: Color) -> Self {
        (value.0, value.1, value.2, value.3)
    }
}

impl From<Color> for (u8, u8, u8) {
    fn from(value: Color) -> Self {
        (value.0, value.1, value.2)
    }
}

impl From<(u8, u8, u8, u8)> for Color {
    fn from(value: (u8, u8, u8, u8)) -> Self {
        Color(value.0, value.1, value.2, value.3)
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from(value: (u8, u8, u8)) -> Self {
        Color(value.0, value.1, value.2, 255)
    }
}

impl From<Color> for [u8; 4] {
    fn from(value: Color) -> Self {
        [value.0, value.1, value.2, value.3]
    }
}

impl From<Color> for [u8; 3] {
    fn from(value: Color) -> Self {
        [value.0, value.1, value.2]
    }
}

impl From<[u8; 4]> for Color {
    fn from(value: [u8; 4]) -> Self {
        Color(value[0], value[1], value[2], value[3])
    }
}

impl From<[u8; 3]> for Color {
    fn from(value: [u8; 3]) -> Self {
        Color(value[0], value[1], value[2], 255)
    }
}

impl FromStr for Color {
    type Err = String;
    fn from_str(mut color: &str) -> Result<Self, Self::Err> {
        color = color.trim().strip_prefix('#').unwrap_or(color);
        let color = match color.len() {
            // TODO: use repeat_n once our MSRV is bumped to 1.82
            3 => color
                .chars()
                .flat_map(|c| std::iter::repeat(c).take(2))
                .chain(std::iter::repeat('f').take(2))
                .collect(),
            6 => format!("{color}FF"),
            8 => color.to_string(),
            _ => {
                return Err(
                    "Invalid hex color length, must be either 3, 6 or 8, for example: #fff, #ffffff, or #ffffffff"
                        .into(),
                );
            }
        };

        let r = u8::from_str_radix(&color[0..2], 16).map_err(|e| e.to_string())?;
        let g = u8::from_str_radix(&color[2..4], 16).map_err(|e| e.to_string())?;
        let b = u8::from_str_radix(&color[4..6], 16).map_err(|e| e.to_string())?;
        let a = u8::from_str_radix(&color[6..8], 16).map_err(|e| e.to_string())?;

        Ok(Color(r, g, b, a))
    }
}

fn default_alpha() -> u8 {
    255
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
enum InnerColor {
    /// Color hex string, for example: #fff, #ffffff, or #ffffffff.
    String(String),
    /// Array of RGB colors. Each value has minimum of 0 and maximum of 255.
    Rgb((u8, u8, u8)),
    /// Array of RGBA colors. Each value has minimum of 0 and maximum of 255.
    Rgba((u8, u8, u8, u8)),
    /// Object of red, green, blue, alpha color values. Each value has minimum of 0 and maximum of 255.
    RgbaObject {
        red: u8,
        green: u8,
        blue: u8,
        #[serde(default = "default_alpha")]
        alpha: u8,
    },
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let color = InnerColor::deserialize(deserializer)?;
        let color = match color {
            InnerColor::String(string) => string.parse().map_err(serde::de::Error::custom)?,
            InnerColor::Rgb(rgb) => Color(rgb.0, rgb.1, rgb.2, 255),
            InnerColor::Rgba(rgb) => rgb.into(),
            InnerColor::RgbaObject {
                red,
                green,
                blue,
                alpha,
            } => Color(red, green, blue, alpha),
        };

        Ok(color)
    }
}

/* #[cfg(feature = "schema")]
impl schemars::JsonSchema for Color {
  fn schema_name() -> String {
    "Color".to_string()
  }

  fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
    let mut schema = schemars::schema_for!(InnerColor).schema;
    schema.metadata = None; // Remove `title: InnerColor` from schema

    // add hex color pattern validation
    let any_of = schema.subschemas().any_of.as_mut().unwrap();
    let schemars::schema::Schema::Object(str_schema) = any_of.first_mut().unwrap() else {
      unreachable!()
    };
    str_schema.string().pattern = Some("^#?([A-Fa-f0-9]{3}|[A-Fa-f0-9]{6}|[A-Fa-f0-9]{8})$".into());

    schema.into()
  }
}
 */

/// The window effects configuration object
#[skip_serializing_none]
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowEffectsConfig {
    /// List of Window effects to apply to the Window.
    /// Conflicting effects will apply the first one and ignore the rest.
    pub effects: Vec<WindowEffect>,
    /// Window effect state **macOS Only**
    pub state: Option<WindowEffectState>,
    /// Window effect corner radius **macOS Only**
    pub radius: Option<f64>,
    /// Window effect color. Affects [`WindowEffect::Blur`] and [`WindowEffect::Acrylic`] only
    /// on Windows 10 v1903+. Doesn't have any effect on Windows 7 or Windows 11.
    pub color: Option<Color>,
}

/// Background throttling policy.
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum BackgroundThrottlingPolicy {
    /// A policy where background throttling is disabled
    Disabled,
    /// A policy where a web view that's not in a window fully suspends tasks. This is usually the default behavior in case no policy is set.
    Suspend,
    /// A policy where a web view that's not in a window limits processing, but does not fully suspend tasks.
    Throttle,
}

/// An URL to open on a Tauri webview window.
#[derive(PartialEq, Eq, Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
#[non_exhaustive]
pub enum WebviewUrl {
    /// An external URL. Must use either the `http` or `https` schemes.
    External(Url),
    /// The path portion of an app URL.
    /// For instance, to load `tauri://localhost/users/john`,
    /// you can simply provide `users/john` in this configuration.
    App(PathBuf),
    /// A custom protocol url, for example, `doom://index.html`
    CustomProtocol(Url),
}

impl<'de> Deserialize<'de> for WebviewUrl {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum WebviewUrlDeserializer {
            Url(Url),
            Path(PathBuf),
        }

        match WebviewUrlDeserializer::deserialize(deserializer)? {
            WebviewUrlDeserializer::Url(u) => {
                if u.scheme() == "https" || u.scheme() == "http" {
                    Ok(Self::External(u))
                } else {
                    Ok(Self::CustomProtocol(u))
                }
            }
            WebviewUrlDeserializer::Path(p) => Ok(Self::App(p)),
        }
    }
}

impl fmt::Display for WebviewUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::External(url) | Self::CustomProtocol(url) => {
                write!(f, "{url}")
            }
            Self::App(path) => write!(f, "{}", path.display()),
        }
    }
}

impl Default for WebviewUrl {
    fn default() -> Self {
        Self::App("index.html".into())
    }
}
