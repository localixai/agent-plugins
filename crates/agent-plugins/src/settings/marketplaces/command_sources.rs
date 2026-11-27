//! Port of `src/settings/marketplaces/command-sources.ts`, switched.
//!
//! `disableCommandPluginSources`: whether Claude Code may run a command a marketplace declares
//! (settings-reference.md, `disableCommandPluginSources` and "What runs under `allowManagedHooksOnly`";
//! plugins/host-marketplace.md, "When Claude Code skips a headersHelper command"). The kit returns the
//! decision; running, installing or loading a command-sourced plugin stays the host's. Its policy
//! mixes data members with a method (`headersHelper`), which no answer of the boundary carries (a
//! handle has methods only), so the package keeps its TypeScript: nothing of it is exported.

use serde::Serialize;

use crate::js::{JsValue, is_truthy};
use crate::registry::{Function, Port};
use crate::settings::layers::PolicyScope;
use crate::settings::view::PolicyView;

/// The key that blocks a command: the explicit one, or the one that decides while it is unset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandBlocker {
    /// `disableCommandPluginSources`.
    DisableCommandPluginSources,
    /// `allowManagedHooksOnly`.
    AllowManagedHooksOnly,
}

/// Whether a command may run, and why not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandVerdict {
    /// It runs.
    Run,
    /// It does not, for this key.
    Blocked(CommandBlocker),
}

/// What the policy says of command sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandSourcePolicy {
    /// Command sources are blocked, by either key.
    pub disabled: bool,
    /// A plugin with a `command` source: its command never runs, it is not installed or updated, and one
    /// already installed stops loading, a plugin force-enabled in managed `enabledPlugins` included.
    pub plugin_source: CommandVerdict,
}

impl CommandSourcePolicy {
    /// A marketplace's or an inline plugin entry's `headersHelper`, given the scope of the file that
    /// declares the marketplace. Blocked with the command sources, "except for a marketplace that managed
    /// settings themselves declare".
    #[must_use]
    pub fn headers_helper(&self, declared_in: PolicyScope) -> CommandVerdict {
        if declared_in == PolicyScope::Managed {
            ALLOWED
        } else {
            self.plugin_source
        }
    }
}

/// Which key blocks, if any. The explicit value decides when set, `false` included; unset, the
/// organization's hook restriction decides ("Default: unset, so Claude Code follows
/// `allowManagedHooksOnly`"). `disableAllHooks` is not consulted, not even from the SDK's
/// `managedSettings`: it "doesn't by itself disable command sources".
async fn blocker(view: &dyn PolicyView) -> Option<CommandBlocker> {
    if let Some(explicit) = view.get("disableCommandPluginSources").await
        && !explicit.is_undefined()
    {
        return is_truthy(&explicit).then_some(CommandBlocker::DisableCommandPluginSources);
    }
    (view.get("allowManagedHooksOnly").await == Some(JsValue::Bool(true)))
        .then_some(CommandBlocker::AllowManagedHooksOnly)
}

const ALLOWED: CommandVerdict = CommandVerdict::Run;

/// The policy the view's settings make.
pub async fn create_command_source_policy(view: &dyn PolicyView) -> CommandSourcePolicy {
    let reason = blocker(view).await;
    let blocked_verdict = reason.map_or(ALLOWED, CommandVerdict::Blocked);
    CommandSourcePolicy {
        disabled: reason.is_some(),
        plugin_source: blocked_verdict,
    }
}

/// `CommandVerdict` as the package writes it: `{ run: true }` or `{ run: false, reason }`.
#[derive(Serialize)]
struct VerdictShape {
    run: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<CommandBlocker>,
}

impl Serialize for CommandVerdict {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let shape = match *self {
            Self::Run => VerdictShape {
                run: true,
                reason: None,
            },
            Self::Blocked(reason) => VerdictShape {
                run: false,
                reason: Some(reason),
            },
        };
        shape.serialize(serializer)
    }
}

struct HeadersHelper(CommandSourcePolicy);

crate::registry::methods! {
    HeadersHelper {
        "call" => ok |this, declared_in: PolicyScope| this.0.headers_helper(declared_in);
    }
}

/// The policy as the host holds it: `headersHelper` a function of the core's.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HostPolicy {
    disabled: bool,
    plugin_source: CommandVerdict,
    headers_helper: Function,
}

async fn host_policy(view: Port) -> HostPolicy {
    let policy = create_command_source_policy(&view).await;
    HostPolicy {
        disabled: policy.disabled,
        plugin_source: policy.plugin_source,
        headers_helper: Function::new(HeadersHelper(policy)),
    }
}

crate::registry::exports! {
    "settings/marketplaces/command-sources" {
        "createCommandSourcePolicy" => async ok |view: Port| host_policy(view).await;
    }
}
