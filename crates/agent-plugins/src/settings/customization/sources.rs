//! Ported from `src/settings/customization/sources.ts`; only the core uses it now, so it has no wrapper.
//!
//! Which origins each locked surface still loads: one table per surface, typed over that surface's
//! own origins, from the **Stops** and **Keeps** lists of its sub-key entry in
//! settings-reference.md. A host's discovery (`~/.claude/skills`, `.claude/agents`, `.mcp.json`,
//! `--add-dir`) asks per candidate.
//!
//! `userConfig` is the user-scope servers and `localConfig` the local-scope ones (the default of
//! `claude mcp add`): both live in `~/.claude.json` (mcp.md § Scopes), which the lock stops loading
//! from. The hooks of every origin the hook policy knows: the same vocabulary, one decision about
//! it.

use serde::Deserialize;

use crate::js::JsValue;
use crate::ports::now;
use crate::settings::hooks::origin_stops::stops_of;
use crate::settings::hooks::origins::{FrontmatterFrom, PolicyHookSource};

/// When the lock stops an origin: with its own surface locked, with any surface locked (a
/// skills-directory plugin, skipped whole "when any one kind is locked", plugins/loading.md), or
/// never (managed, bundled and the plugins a marketplace delivers).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Blocks {
    WhenSurfaceLocked,
    WhenAnyLocked,
    Never,
}

/// The surfaces locked (`LockedSurfaces`).
type LockedSurfaces = [String];

fn stops(blocks: Blocks, locked: &LockedSurfaces, surface: &str) -> bool {
    match blocks {
        Blocks::WhenSurfaceLocked => locked.iter().any(|one| one == surface),
        Blocks::WhenAnyLocked => !locked.is_empty(),
        Blocks::Never => false,
    }
}

const SKILLS: &[(&str, Blocks)] = &[
    ("userSkills", Blocks::WhenSurfaceLocked),
    ("projectSkills", Blocks::WhenSurfaceLocked),
    ("userCommands", Blocks::WhenSurfaceLocked),
    ("projectCommands", Blocks::WhenSurfaceLocked),
    ("addDir", Blocks::WhenSurfaceLocked),
    ("synced", Blocks::WhenSurfaceLocked),
    ("bundled", Blocks::Never),
    ("managedDirectory", Blocks::Never),
    ("plugin", Blocks::Never),
    ("skillsDirPlugin", Blocks::WhenAnyLocked),
];

const AGENTS: &[(&str, Blocks)] = &[
    ("userAgents", Blocks::WhenSurfaceLocked),
    ("projectAgents", Blocks::WhenSurfaceLocked),
    ("addDir", Blocks::WhenSurfaceLocked),
    ("builtIn", Blocks::Never),
    ("managedDirectory", Blocks::Never),
    ("plugin", Blocks::Never),
    ("skillsDirPlugin", Blocks::WhenAnyLocked),
];

const MCP: &[(&str, Blocks)] = &[
    ("userConfig", Blocks::WhenSurfaceLocked),
    ("localConfig", Blocks::WhenSurfaceLocked),
    ("projectFile", Blocks::WhenSurfaceLocked),
    ("managedFile", Blocks::Never),
    ("managedSettings", Blocks::Never),
    ("plugin", Blocks::Never),
    ("skillsDirPlugin", Blocks::WhenAnyLocked),
];

/// An origin of a table's surface loads unless its row stops it; an origin the table does not
/// name has no row, so nothing stops it (the package's typed tables admit no other).
fn table_loads(
    surface: &str,
    table: &[(&str, Blocks)],
    locked: &LockedSurfaces,
    origin: &JsValue,
) -> bool {
    let row = table
        .iter()
        .find(|(name, _)| origin.as_str() == Some(*name));
    !row.is_some_and(|(_, blocks)| stops(*blocks, locked, surface))
}

/// The skills-directory plugin a hook belongs to, itself or through the skill that declares it.
fn owned_by_skills_dir_plugin(origin: &PolicyHookSource) -> bool {
    if let PolicyHookSource::Plugin(plugin)
    | PolicyHookSource::Frontmatter {
        from: FrontmatterFrom::Plugin { plugin },
        ..
    } = origin
    {
        plugin.skills_dir_plugin
    } else {
        false
    }
}

/// The hooks surface asks the hook policy's own table what the lock stops (settings files,
/// frontmatter of the user's, the project's and added directories' skills and agents,
/// skills-directory plugins), so the two never describe one origin two ways; a skills-directory
/// plugin also stops under any other lock.
fn hooks_loads(locked: &LockedSurfaces, origin: &JsValue) -> bool {
    let Ok(origin) = PolicyHookSource::deserialize(origin.clone()) else {
        return true;
    };
    let plugin_only = now(stops_of(&origin, &|_: &str| false)).is_some_and(|s| s.plugin_only);
    !((locked.iter().any(|one| one == "hooks") && plugin_only)
        || (!locked.is_empty() && owned_by_skills_dir_plugin(&origin)))
}

/// Whether a customization of `surface` from `origin` loads under these locks.
#[must_use]
pub fn loads(locked: &LockedSurfaces, surface: &str, origin: &JsValue) -> bool {
    match surface {
        "skills" => table_loads("skills", SKILLS, locked, origin),
        "agents" => table_loads("agents", AGENTS, locked, origin),
        "hooks" => hooks_loads(locked, origin),
        "mcp" => table_loads("mcp", MCP, locked, origin),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skills_dir_plugin_stops_under_any_lock() {
        let locked = vec!["mcp".to_owned()];
        assert!(!loads(&locked, "skills", &JsValue::from("skillsDirPlugin")));
        assert!(loads(&locked, "skills", &JsValue::from("userSkills")));
        assert!(!loads(&locked, "mcp", &JsValue::from("projectFile")));
    }
}
