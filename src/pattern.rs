use std::sync::Arc;

use serde::Serialize;
use serialize_to_javascript::{Template, default_template};

/// The domain of the isolation iframe source.

pub const ISOLATION_IFRAME_SRC_DOMAIN: &str = "localhost";

/// An application pattern.
#[derive(Debug)]
pub enum Pattern {
    /// The brownfield pattern.
    Brownfield,
    /*
    /// Isolation pattern. Recommended for security purposes.
    Isolation {
        /// The HTML served on `isolation://index.html`.
        assets: Arc<EmbeddedAssets>,

        /// The schema used for the isolation frames.
        schema: String,

        /// A random string used to ensure that the message went through the isolation frame.
        ///
        /// This should be regenerated at runtime.
        key: String,

        /// Cryptographically secure keys
        crypto_keys: Box<tauri_utils::pattern::isolation::Keys>,
      }, */
}

/// The shape of the JavaScript Pattern config
#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase", tag = "pattern")]
pub(crate) enum PatternObject {
    /// Brownfield pattern.
    Brownfield,
    /// Isolation pattern. Recommended for security purposes.
    Isolation {
        /// Which `IsolationSide` this `PatternObject` is getting injected into
        side: IsolationSide,
    },
}

impl From<&Pattern> for PatternObject {
    fn from(pattern: &Pattern) -> Self {
        match pattern {
            Pattern::Brownfield => Self::Brownfield,
            /*       Pattern::Isolation { .. } => Self::Isolation {
              side: IsolationSide::default(),
            }, */
        }
    }
}

/// Where the JavaScript is injected to

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum IsolationSide {
    /// Original frame, the Brownfield application
    #[default]
    Original,
    /// Secure frame, the isolation security application
    #[allow(dead_code)]
    Secure,
}

#[derive(Template)]
#[default_template("../scripts/pattern.js")]
pub(crate) struct PatternJavascript {
    pub(crate) pattern: PatternObject,
}

pub(crate) fn format_real_schema(schema: &str, https: bool) -> String {
    if cfg!(windows) || cfg!(target_os = "android") {
        let scheme = if https { "https" } else { "http" };
        format!("{scheme}://{schema}.{ISOLATION_IFRAME_SRC_DOMAIN}/")
    } else {
        format!("{schema}://{ISOLATION_IFRAME_SRC_DOMAIN}/")
    }
}
