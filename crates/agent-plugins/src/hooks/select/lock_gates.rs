//! Port of `src/hooks/select/lock-gates.ts`, switched.
//!
//! The two managed settings that narrow where hooks may come from (hooks.md, "Hook locations" and
//! "Limits on skill hooks"; settings-reference.md, "allowManagedHooksOnly" and
//! "strictPluginOnlyCustomization.hooks"). Each is a gate per source kind.

use crate::hooks::contracts::{HookSession, HookSourceKind, RegisteredHandler};

fn managed(entry: &RegisteredHandler) -> bool {
    entry.source.managed == Some(true)
}

/// Hooks of a plugin force-enabled in managed `enabledPlugins`, matched on its full id.
fn force_enabled(entry: &RegisteredHandler, session: &HookSession) -> bool {
    entry.plugin_id.as_ref().is_some_and(|plugin_id| {
        session
            .force_enabled_plugins
            .as_ref()
            .is_some_and(|plugins| plugins.members.contains(plugin_id))
    })
}

/// `allowManagedHooksOnly: true`: "Your user, project, local, and plugin hooks are blocked. Hooks
/// from plugins force-enabled in managed settings `enabledPlugins` are exempt"; managed hooks and
/// the SDK's in-process hooks (the `session` kind) run; "hooks declared in agent frontmatter" are
/// blocked. Judgment (spec/judgments.md): a skill's frontmatter hooks follow the same split as the
/// rest, so they run only when the skill is managed or comes from a force-enabled plugin.
fn managed_only(entry: &RegisteredHandler, session: &HookSession) -> bool {
    match entry.source.kind {
        HookSourceKind::Settings => managed(entry),
        HookSourceKind::Skill => managed(entry) || force_enabled(entry, session),
        HookSourceKind::Agent => false,
        HookSourceKind::Plugin => force_enabled(entry, session),
        HookSourceKind::Session => true,
    }
}

/// The settings files a user or project owns: the ones a plugin-only lock silences.
const USER_OWNED_SCOPES: [&str; 3] = ["user", "project", "local"];

/// "Hooks from user, project, and local `settings.json`" and the frontmatter hooks of skills and
/// agents "from the sources that a `skills` or `agents` lock would block" stop; "hooks in managed
/// settings, and hooks from plugins" keep running, and a skill keeps its frontmatter hooks only if
/// it is "a plugin skill, a bundled skill, or a skill in the managed settings directory". A source
/// with no user or project scope (a bundled skill, a `--settings` file) is not the user's own.
fn not_user_owned(entry: &RegisteredHandler) -> bool {
    let source = &entry.source;
    if source.managed == Some(true) || source.plugin_name.is_some() {
        return true;
    }
    source
        .scope
        .as_ref()
        .is_none_or(|scope| !USER_OWNED_SCOPES.contains(&scope.as_str()))
}

/// Whether a plugin-only lock on `hooks` leaves the handler's source alone. It reads the source, not
/// the session, so a host can tell which registered handlers a lock silences.
#[must_use]
pub fn plugin_only_allows(entry: &RegisteredHandler) -> bool {
    match entry.source.kind {
        HookSourceKind::Settings | HookSourceKind::Skill | HookSourceKind::Agent => {
            not_user_owned(entry)
        }
        HookSourceKind::Plugin | HookSourceKind::Session => true,
    }
}

/// Whether neither managed lock the session has set keeps the handler's source from running.
#[must_use]
pub fn passes_locks(entry: &RegisteredHandler, session: &HookSession) -> bool {
    (session.allow_managed_hooks_only != Some(true) || managed_only(entry, session))
        && (session.plugin_only_hooks != Some(true) || plugin_only_allows(entry))
}

crate::registry::exports! {
    "hooks/select/lock-gates" {
        "pluginOnlyAllows" => ok |entry: RegisteredHandler| plugin_only_allows(&entry);
        "passesLocks" => ok |entry: RegisteredHandler, session: HookSession| passes_locks(&entry, &session);
    }
}
