//! Port of `src/settings/hooks/http-policy.ts`, switched.
//!
//! The two allowlists of HTTP hooks (settings-reference.md, `allowedHttpHookUrls` and
//! `httpHookAllowedEnvVars`). Both merge across files, both are read from "hooks from every
//! source, including managed settings", so the check takes a URL and a hook's own list and no
//! origin.

use indexmap::IndexSet;
use serde::Serialize;

use crate::js::JsValue;
use crate::registry::{Handle, Port};
use crate::settings::hooks::url_pattern::match_url_pattern;
use crate::settings::view::PolicyView;

/// Whether an HTTP hook runs, and which environment variables its headers may interpolate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum HttpHookVerdict {
    /// It runs.
    Run {
        /// Always `true`.
        run: bool,
        /// The variables its headers may interpolate.
        interpolate: Vec<String>,
    },
    /// It is blocked.
    Blocked {
        /// Always `false`.
        run: bool,
        /// Why.
        message: &'static str,
    },
}

/// Judgment (spec/judgments.md): the docs say "blocks the rest without running them" and give no
/// text. The message names the key and not the URL, which can carry a token.
const BLOCKED_MESSAGE: &str =
    "HTTP hook blocked: its URL matches no pattern in allowedHttpHookUrls, so the hook did not run";

fn unique(names: &[String]) -> IndexSet<&String> {
    names.iter().collect()
}

fn strings(value: Option<JsValue>) -> Option<Vec<String>> {
    value.map(|value| {
        value
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// The two lists of the view, read once.
#[derive(Debug)]
pub struct HttpHookUrlPolicy {
    urls: Option<Vec<String>>,
    variables: Option<Vec<String>>,
}

impl HttpHookUrlPolicy {
    /// Whether an HTTP hook runs, and which environment variables its headers may interpolate.
    /// `hook_allowed_env_vars` is the hook's own `allowedEnvVars`.
    #[must_use]
    pub fn check(&self, url: &str, hook_allowed_env_vars: &[String]) -> HttpHookVerdict {
        if let Some(urls) = &self.urls
            && !urls.iter().any(|pattern| match_url_pattern(pattern, url))
        {
            return HttpHookVerdict::Blocked {
                run: false,
                message: BLOCKED_MESSAGE,
            };
        }
        // A hook can use a variable only if both its own list and the key name it.
        let interpolate = unique(hook_allowed_env_vars)
            .into_iter()
            .filter(|name| {
                self.variables
                    .as_ref()
                    .is_none_or(|variables| variables.contains(*name))
            })
            .cloned()
            .collect();
        HttpHookVerdict::Run {
            run: true,
            interpolate,
        }
    }
}

/// The policy over `view`.
pub async fn create_http_hook_policy(view: &dyn PolicyView) -> HttpHookUrlPolicy {
    let urls = strings(view.get("allowedHttpHookUrls").await);
    let variables = strings(view.get("httpHookAllowedEnvVars").await);
    HttpHookUrlPolicy { urls, variables }
}

crate::registry::methods! {
    HttpHookUrlPolicy {
        "check" => ok |this, url: String, hook_allowed_env_vars: Vec<String>|
            this.check(&url, &hook_allowed_env_vars);
    }
}

crate::registry::exports! {
    "settings/hooks/http-policy" {
        "createHttpHookPolicy" => async ok |view: Port|
            Handle::new(create_http_hook_policy(&view).await);
    }
}
