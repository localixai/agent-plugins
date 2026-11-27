//! Ported from `src/settings/hooks/origin-stops.ts`; only the core uses it now, so it has no wrapper.
//!
//! Which lock stops a hook of each origin: one registry typed over the origin kinds, so a new kind
//! fails to compile until it says what the locks do to it. `disableAllHooks` is two columns
//! because its reach "depends on which file carries the key" (settings-reference.md).

use super::origins::{
    ForceEnabled, FrontmatterFrom, HookFrontmatterOwner, PluginHookSource, PolicyHookSource, Stops,
};
use crate::settings::plugins::plugin_id::parse_plugin_id;

const NEVER: Stops = Stops {
    managed_disable: false,
    other_disable: false,
    managed_only: false,
    plugin_only: false,
    command_source: false,
};

/// A hook that every lock but the one on `strictPluginOnlyCustomization.hooks` stops: the user's
/// own.
const OWN: Stops = Stops {
    managed_disable: true,
    other_disable: true,
    managed_only: true,
    plugin_only: true,
    ..NEVER
};

/// The built-in plugins ("Mods built into Claude Code") are not stopped by any of these settings.
fn is_builtin(id: &str) -> bool {
    parse_plugin_id(id).is_some_and(|parsed| parsed.origin == "builtin")
}

/// A plugin's hooks. Under `allowManagedHooksOnly` and a `disableAllHooks` outside managed
/// settings they run only when managed settings force-enable the plugin, and a mod only when it
/// also counts as the organization's ("A mod in such a plugin loads only when it counts as your
/// organization's").
async fn plugin_stops(plugin: &PluginHookSource, force_enabled: &dyn ForceEnabled) -> Stops {
    if is_builtin(&plugin.id) {
        return NEVER;
    }
    let trusted =
        force_enabled.force_enabled(&plugin.id).await && (!plugin.is_mod || plugin.is_org_mod);
    Stops {
        managed_disable: true,
        other_disable: !trusted,
        managed_only: !trusted,
        plugin_only: plugin.skills_dir_plugin,
        command_source: plugin.command_source,
    }
}

/// Frontmatter hooks of an agent are blocked under `allowManagedHooksOnly` whatever the source.
const fn agent_blocked(owner: HookFrontmatterOwner) -> bool {
    match owner {
        HookFrontmatterOwner::Skill | HookFrontmatterOwner::Command => false,
        HookFrontmatterOwner::Agent => true,
    }
}

/// A skill, command or agent bundled with Claude Code.
const OUR_OWN: Stops = Stops {
    managed_disable: true,
    managed_only: true,
    ..NEVER
};

/// A skill, command or agent of the managed policy directory. Judgment: its frontmatter hooks are
/// the organization's, so under `allowManagedHooksOnly` ("managed hooks run") a skill's and a
/// command's keep running, as the plugin-only lock keeps "a skill in the managed settings
/// directory" (hooks.md, "Limits on skill hooks"); an agent's are blocked whatever the source.
const fn managed_directory(owner: HookFrontmatterOwner) -> Stops {
    Stops {
        managed_disable: true,
        managed_only: agent_blocked(owner),
        ..NEVER
    }
}

async fn from(
    owner: HookFrontmatterOwner,
    from: &FrontmatterFrom,
    force_enabled: &dyn ForceEnabled,
) -> Stops {
    match from {
        FrontmatterFrom::User
        | FrontmatterFrom::Project
        | FrontmatterFrom::AddDir
        | FrontmatterFrom::Synced => OWN,
        FrontmatterFrom::Managed => managed_directory(owner),
        FrontmatterFrom::Bundled => OUR_OWN,
        FrontmatterFrom::Plugin { plugin } => {
            let stops = plugin_stops(plugin, force_enabled).await;
            Stops {
                managed_only: stops.managed_only || agent_blocked(owner),
                ..stops
            }
        }
    }
}

/// Which locks stop a hook of this origin.
pub async fn stops_of(origin: &PolicyHookSource, force_enabled: &dyn ForceEnabled) -> Stops {
    match origin {
        PolicyHookSource::ManagedSettings => Stops {
            managed_disable: true,
            ..NEVER
        },
        PolicyHookSource::SdkCallback => NEVER,
        PolicyHookSource::Settings { .. } => OWN,
        PolicyHookSource::Plugin(plugin) => plugin_stops(plugin, force_enabled).await,
        PolicyHookSource::Frontmatter {
            owner,
            from: source,
        } => from(*owner, source, force_enabled).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::now;

    #[test]
    fn a_plugin_skill_agent_is_blocked_under_managed_only() {
        let plugin = PluginHookSource {
            id: "p@m".to_owned(),
            skills_dir_plugin: false,
            command_source: false,
            is_mod: false,
            is_org_mod: false,
        };
        let origin = PolicyHookSource::Frontmatter {
            owner: HookFrontmatterOwner::Agent,
            from: FrontmatterFrom::Plugin { plugin },
        };
        let stops = now(stops_of(&origin, &|_: &str| true));
        assert_eq!(stops.map(|stops| stops.managed_only), Some(true));
        assert_eq!(stops.map(|stops| stops.other_disable), Some(false));
    }
}
