//! Port of `src/hooks/select/gates.ts`, switched.
//!
//! What keeps a registered handler from running whatever the event says: `disableAllHooks`, workspace
//! trust, a plugin that is not enabled and the two managed locks (`lock-gates.ts`). Each is the docs'
//! rule, applied to the handler's source.

use crate::hooks::contracts::{HookSession, HookSource, HookSourceKind, RegisteredHandler};

use super::lock_gates::passes_locks;

/// hooks.md, "Disable or remove hooks": `disableAllHooks` left after precedence turns off every hook
/// but the managed ones ("`disableAllHooks` set in user, project, or local settings can't disable
/// those managed hooks. Only `disableAllHooks` set at the managed settings level can").
fn hooks_enabled(source: &HookSource, session: &HookSession) -> bool {
    if session.disable_all_hooks.managed_locked {
        return false;
    }
    source.managed == Some(true) || !session.disable_all_hooks.effective
}

/// A settings-file hook waits for workspace trust; so does a project skill's, by the same rule.
const fn settings_trust(_source: &HookSource, session: &HookSession) -> bool {
    session.trusted
}

/// A rule that applies to a project-scoped source only.
fn project_only(
    source: &HookSource,
    rule: impl Fn(&HookSession) -> bool,
    session: &HookSession,
) -> bool {
    source.scope.as_deref() != Some("project") || rule(session)
}

/// Per source kind (hooks.md, "Workspace trust" and "Hooks in skills and agents"):
/// - settings: "Claude Code holds back hooks from every settings file ... until you accept the
///   workspace trust dialog"; a `-p` or SDK session counts as trusted (`session.trusted`);
/// - a project skill's frontmatter hooks "follow the same workspace trust rule";
/// - a project subagent's frontmatter hooks need the dialog for the folder itself, and "a `-p`
///   session doesn't count as trusted"; user-level agents are exempt;
/// - a plugin's hooks run "when plugin is enabled".
fn source_gate(source: &HookSource, session: &HookSession) -> bool {
    match source.kind {
        HookSourceKind::Settings => settings_trust(source, session),
        HookSourceKind::Skill => project_only(source, |session| session.trusted, session),
        HookSourceKind::Agent => project_only(
            source,
            |session| session.folder_trust_accepted == Some(true),
            session,
        ),
        HookSourceKind::Plugin => session.enabled_plugins.as_ref().is_none_or(|enabled| {
            source
                .plugin_name
                .as_ref()
                .is_some_and(|name| enabled.members.contains(name))
        }),
        HookSourceKind::Session => true,
    }
}

/// Whether the handler may run in this session at all.
#[must_use]
pub fn passes_gates(entry: &RegisteredHandler, session: &HookSession) -> bool {
    hooks_enabled(&entry.source, session)
        && source_gate(&entry.source, session)
        && passes_locks(entry, session)
}

crate::registry::exports! {
    "hooks/select/gates" {
        "passesGates" => ok |entry: RegisteredHandler, session: HookSession| passes_gates(&entry, &session);
    }
}
