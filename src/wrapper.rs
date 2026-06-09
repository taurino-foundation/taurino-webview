use std::{
    borrow::Cow,
    path::{Component, Path},
};

use dpi::{Position, Size};
use serde::Serialize;

/// A rectangular region.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Rect {
    /// Rect position.
    pub position: dpi::Position,
    /// Rect size.
    pub size: dpi::Size,
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            position: Position::Logical((0, 0).into()),
            size: Size::Logical((0, 0).into()),
        }
    }
}

/// A rectangular region in physical pixels.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct PhysicalRect<P: dpi::Pixel, S: dpi::Pixel> {
    /// Rect position.
    pub position: dpi::PhysicalPosition<P>,
    /// Rect size.
    pub size: dpi::PhysicalSize<S>,
}

impl<P: dpi::Pixel, S: dpi::Pixel> Default for PhysicalRect<P, S> {
    fn default() -> Self {
        Self {
            position: (0, 0).into(),
            size: (0, 0).into(),
        }
    }
}

/// A rectangular region in logical pixels.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct LogicalRect<P: dpi::Pixel, S: dpi::Pixel> {
    /// Rect position.
    pub position: dpi::LogicalPosition<P>,
    /// Rect size.
    pub size: dpi::LogicalSize<S>,
}

impl<P: dpi::Pixel, S: dpi::Pixel> Default for LogicalRect<P, S> {
    fn default() -> Self {
        Self {
            position: (0, 0).into(),
            size: (0, 0).into(),
        }
    }
}

pub struct RectWrapper(pub wry::Rect);
impl From<Rect> for RectWrapper {
    fn from(value: Rect) -> Self {
        RectWrapper(wry::Rect {
            position: value.position,
            size: value.size,
        })
    }
}

/// Assets iterator.
pub type AssetsIter<'a> =
    dyn Iterator<Item = (Cow<'a, str>, Cow<'a, [u8]>)> + 'a;

/// Represent an asset file path in a normalized way.
///
/// The following rules are enforced and added if needed:
/// * Unix path component separators
/// * Has a root directory
/// * No trailing slash - directories are not included in assets
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct AssetKey(String);

impl From<AssetKey> for String {
    fn from(key: AssetKey) -> Self {
        key.0
    }
}

impl AsRef<str> for AssetKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<P: AsRef<Path>> From<P> for AssetKey {
    fn from(path: P) -> Self {
        // TODO: change this to utilize `Cow` to prevent allocating an intermediate `PathBuf` when not necessary
        let path = path.as_ref().to_owned();

        // add in root to mimic how it is used from a server url
        let path = if path.has_root() {
            path
        } else {
            Path::new(&Component::RootDir).join(path)
        };

        let buf = if cfg!(windows) {
            let mut buf = String::new();
            for component in path.components() {
                match component {
                    Component::RootDir => buf.push('/'),
                    Component::CurDir => buf.push_str("./"),
                    Component::ParentDir => buf.push_str("../"),
                    Component::Prefix(prefix) => {
                        buf.push_str(&prefix.as_os_str().to_string_lossy())
                    }
                    Component::Normal(s) => {
                        buf.push_str(&s.to_string_lossy());
                        buf.push('/')
                    }
                }
            }

            // remove the last slash
            if buf != "/" {
                buf.pop();
            }

            buf
        } else {
            path.to_string_lossy().to_string()
        };

        AssetKey(buf)
    }
}
