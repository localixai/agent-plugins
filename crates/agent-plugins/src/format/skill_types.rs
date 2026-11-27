//! Port of `src/format/skill-types.ts`, switched.
//!
//! The model of a skill and of a command (skills.md, "Configure skills"; plugins/components.md,
//! "Skills" and "Commands"). A command file is "the older format" and "supports the same
//! frontmatter" as a skill (skills.md, "Choose where skills load"), so both carry one
//! `InvocableFrontmatter`. The core holds the two constants; the types stay in the TypeScript.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// The shells `shell` accepts (skills.md, frontmatter reference: "`bash` (default) or `powershell`").
pub const SKILL_SHELLS: &[&str] = &["bash", "powershell"];

/// What differs per origin; a registry typed over the union (K1), read by the expansion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per documented behaviour, as the TypeScript interface has"
)]
pub struct OriginTraits {
    /// `${CLAUDE_PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_DATA}`: "Substituted only in plugin skills".
    pub substitutes_plugin_variables: bool,
    /// `${CLAUDE_PROJECT_DIR}` and `${CLAUDE_SESSION_ID}`. A synced skill is not substituted these
    /// in a session on the owner's machine: "the two placeholders reach Claude as literal text"
    /// (skills.md, "How Claude Code handles the body of a synced skill"). An isolated session (cloud,
    /// Cowork) is the host's decision, as for `runs_shell`.
    pub substitutes_session_variables: bool,
    /// Whether `` !`command` `` blocks run. A skill synced from claude.ai "doesn't run `!` commands"
    /// in a session on the owner's machine (skills.md, "How Claude Code handles the body of a synced
    /// skill"); a cloud or Cowork session is the host's decision, not an origin's.
    pub runs_shell: bool,
    /// Whether `skillOverrides` applies: "Plugin skills are not affected by `skillOverrides`. Manage
    /// those through `/plugin` instead" (skills.md, "Override skill visibility from settings").
    pub honours_skill_overrides: bool,
    /// Whether the files the expanded text names with `@` are attached. A synced skill's body, in a
    /// session on the owner's machine, "doesn't attach the files that `@` references name the way it
    /// does for a local skill" (skills.md, "How Claude Code handles the body of a synced skill"); an
    /// isolated session (cloud, Cowork) keeps the local behaviour, as for `runs_shell`.
    pub attaches_file_references: bool,
}

/// The shell of a `!` command (`shell` of the frontmatter).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillShell {
    /// Bash.
    Bash,
    /// PowerShell.
    Powershell,
}

/// What each origin may do with a skill's text, by the origin's kind.
#[must_use]
pub fn origin_traits() -> IndexMap<&'static str, OriginTraits> {
    IndexMap::from([
        (
            "plugin",
            OriginTraits {
                substitutes_plugin_variables: true,
                substitutes_session_variables: true,
                runs_shell: true,
                honours_skill_overrides: false,
                attaches_file_references: true,
            },
        ),
        (
            "managed",
            OriginTraits {
                substitutes_plugin_variables: false,
                substitutes_session_variables: true,
                runs_shell: true,
                honours_skill_overrides: true,
                attaches_file_references: true,
            },
        ),
        (
            "user",
            OriginTraits {
                substitutes_plugin_variables: false,
                substitutes_session_variables: true,
                runs_shell: true,
                honours_skill_overrides: true,
                attaches_file_references: true,
            },
        ),
        (
            "project",
            OriginTraits {
                substitutes_plugin_variables: false,
                substitutes_session_variables: true,
                runs_shell: true,
                honours_skill_overrides: true,
                attaches_file_references: true,
            },
        ),
        (
            "synced",
            OriginTraits {
                substitutes_plugin_variables: false,
                substitutes_session_variables: false,
                runs_shell: false,
                honours_skill_overrides: true,
                attaches_file_references: false,
            },
        ),
        // "set one skill to `"off"` in `skillOverrides` to hide it" (skills.md, "Remove a skill").
        (
            "bundled",
            OriginTraits {
                substitutes_plugin_variables: false,
                substitutes_session_variables: true,
                runs_shell: true,
                honours_skill_overrides: true,
                attaches_file_references: true,
            },
        ),
    ])
}

crate::registry::exports! {
    "format/skill-types" {
        "SKILL_SHELLS" => ok || SKILL_SHELLS;
        "ORIGIN_TRAITS" => ok origin_traits;
    }
}
