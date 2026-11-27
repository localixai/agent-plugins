//! Port of `src/settings/plugins/sync-plugins.ts` at sha256:baad1fbdf38f.
//!
//! It takes the host's `PolicyView`, so it stays
//! TypeScript in the package: nothing of it is exported.

use crate::js::is_truthy;
use crate::settings::layers::PolicyScope;
use crate::settings::view::PolicyView;

/// What `syncClaudeAiPlugins` asks the host to do; the host does the I/O.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncDecision {
    /// `false`: do not download the plugins enabled on the claude.ai account.
    pub download: bool,
    /// `false`: do not load the plugins already in `~/.claude/plugins/synced/`.
    pub load_synced: bool,
    /// Move the already synced plugins to `~/.claude/plugins/.trash/`.
    pub move_to_trash: bool,
}

/// The two keys that stop the claude.ai sync; they read the same way (settings.md, "Exceptions").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncKey {
    /// `syncClaudeAiPlugins`.
    Plugins,
    /// `syncClaudeAiSkills`.
    Skills,
}

impl SyncKey {
    /// The key's name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Plugins => "syncClaudeAiPlugins",
            Self::Skills => "syncClaudeAiSkills",
        }
    }
}

/// The files whose `false` also moves the synced plugins (settings-reference.md: "user or managed").
const MOVES: [PolicyScope; 2] = [PolicyScope::User, PolicyScope::Managed];

/// Whether Claude Code syncs and loads the plugins of the claude.ai account. Only `false` counts: `true`
/// is the same as unset. A `false` the key's scope line admits (user, local, managed, `--settings`)
/// stops sync wherever it sits, even under a managed `true` (settings.md, "Exceptions to managed
/// settings precedence"). The decision is the key's part only: that a session signs in with a
/// claude.ai account at all (plugins/loading.md, "Sign-in requirements for terminal sync") is the host's.
///
/// Judgment (spec/judgments.md): one `false` in a user or managed file moves the plugins, even when
/// a higher file only stops the sync.
#[must_use]
pub fn sync_decision(view: &dyn PolicyView) -> SyncDecision {
    sync_decision_of(view, SyncKey::Plugins)
}

/// The reading both sync keys share: `syncClaudeAiSkills` has the same scope line, the same
/// honored-only-`false` rule and the same move to the trash (settings-reference.md, skills.md "How
/// synced skills behave"), only into `~/.claude/skills/.trash/`.
#[must_use]
pub fn sync_decision_of(view: &dyn PolicyView, key: SyncKey) -> SyncDecision {
    let off: Vec<_> = view
        .each_from_highest(key.name())
        .into_iter()
        .filter(|holder| !is_truthy(&holder.value))
        .collect();
    if off.is_empty() {
        return SyncDecision {
            download: true,
            load_synced: true,
            move_to_trash: false,
        };
    }
    SyncDecision {
        download: false,
        load_synced: false,
        move_to_trash: off.iter().any(|holder| MOVES.contains(&holder.scope)),
    }
}
