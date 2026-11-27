//! Port of `src/settings/skills/sync-skills.ts`, switched.

use crate::registry::Port;
use crate::settings::plugins::sync_plugins::{SyncDecision, SyncKey, sync_decision_of};
use crate::settings::view::PolicyView;

/// Whether Claude Code syncs and loads the skills of the claude.ai account (settings-reference.md,
/// `syncClaudeAiSkills`; settings.md, "Exceptions to managed settings precedence"). Only `false`
/// counts, from user, local, managed and `--settings` files, also over a managed `true`; in user or
/// managed settings it also moves the synced skills to `~/.claude/skills/.trash/`. That a session
/// signs in with a claude.ai account, and the skills lock of `strictPluginOnlyCustomization`, are
/// not this key's (skills.md, "Where synced skills load").
pub async fn sync_skills_decision(view: &dyn PolicyView) -> SyncDecision {
    sync_decision_of(view, SyncKey::Skills).await
}

crate::registry::exports! {
    "settings/skills/sync-skills" {
        "syncSkillsDecision" => async ok |view: Port| sync_skills_decision(&view).await;
    }
}
