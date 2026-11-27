//! Port of `src/loading/resolve/shell-guard.ts`, switched.
//!
//! A field whose value is passed to a shell refuses `${user_config.*}` (plugins/manifest-reference.md,
//! "Fields that run through a shell"): "A component that references it in one of these fields fails
//! with an error instead of running, because the field's value is passed to a shell that would
//! re-parse the substituted value". The check reads the template, never a value, so it fires even
//! when nothing is configured yet (errors.md, "Plugin command references user_config in a shell
//! command"). Each surface has its own guard (its message and context); this file holds the
//! `headersHelper` guard and the one check every surface calls.

use serde::{Deserialize, Serialize};

use super::shell_guard_hook::{HookGuard, HookGuardContext};
use super::shell_guard_monitor::{MonitorGuard, MonitorGuardContext};
use super::user_config::references_user_config;
use crate::format::issues::{PluginIssue, error};
use crate::js::JsValue;
use crate::js::value::to_js_value;
use crate::ports::BoxFuture;
use crate::registry::{Handle, Port};

/// A surface's guard: the documented message of the surface, quoted from errors.md.
pub trait ShellGuard<Context>: Send + Sync {
    /// The message for a template that references `${user_config.*}`.
    fn message<'a>(&'a self, context: &'a Context) -> BoxFuture<'a, String>;
}

impl ShellGuard<HookGuardContext> for HookGuard {
    fn message<'a>(&'a self, context: &'a HookGuardContext) -> BoxFuture<'a, String> {
        Box::pin(std::future::ready(Self::message(self, context)))
    }
}

impl ShellGuard<MonitorGuardContext> for MonitorGuard {
    fn message<'a>(&'a self, context: &'a MonitorGuardContext) -> BoxFuture<'a, String> {
        Box::pin(std::future::ready(Self::message(self, context)))
    }
}

/// A guard the host holds (its own, or one of the core's handed back): its `message` method.
impl<Context: Serialize + Sync> ShellGuard<Context> for Port {
    fn message<'a>(&'a self, context: &'a Context) -> BoxFuture<'a, String> {
        let argument = to_js_value(context).unwrap_or_default();
        Box::pin(self.method("message", vec![argument]))
    }
}

/// What the `headersHelper` message names.
#[derive(Clone, Debug, Deserialize)]
pub struct HeadersHelperContext {
    /// The MCP server's name.
    pub server: String,
}

/// An MCP `headersHelper`, which runs through a shell (mcp.md, "dynamic headers").
#[derive(Debug)]
pub struct HeadersHelperGuard;

impl HeadersHelperGuard {
    /// The documented message of the surface.
    #[must_use]
    pub fn message(&self, context: &HeadersHelperContext) -> String {
        let server = &context.server;
        format!(
            "headersHelper for MCP server '{server}' references ${{user_config.*}}. The substituted value would be passed to a shell; read the value inside the helper script instead (e.g. from an env var set in the server's \"env\" block)."
        )
    }
}

impl ShellGuard<HeadersHelperContext> for HeadersHelperGuard {
    fn message<'a>(&'a self, context: &'a HeadersHelperContext) -> BoxFuture<'a, String> {
        Box::pin(std::future::ready(Self::message(self, context)))
    }
}

crate::registry::methods! {
    HeadersHelperGuard {
        "message" => ok |this, context: HeadersHelperContext| this.message(&context);
    }
}

/// The guard of an MCP `headersHelper`.
pub const HEADERS_HELPER_GUARD: HeadersHelperGuard = HeadersHelperGuard;

/// The error for a template that references `${user_config.*}`, or `None` when it does not; the
/// guard is asked for its message only then.
pub async fn reject_user_config_in_shell<Context>(
    guard: &dyn ShellGuard<Context>,
    template: &str,
    context: &Context,
    path: &str,
) -> Option<PluginIssue> {
    if !references_user_config(template) {
        return None;
    }
    Some(error(path, &guard.message(context).await))
}

crate::registry::exports! {
    "loading/resolve/shell-guard" {
        "rejectUserConfigInShell" => async ok |guard: Port, template: String, context: JsValue, path: String|
            reject_user_config_in_shell(&guard, &template, &context, &path).await;
        "HEADERS_HELPER_GUARD" => ok || Handle::new(HEADERS_HELPER_GUARD);
    }
}
