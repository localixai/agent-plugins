//! Port of `src/protocol/mods-settings.ts`, switched.
//!
//! The settings and environment variables that affect mods, and where each is read from
//! (plugins/mods/reference.md, "Settings and environment variables", "Where"). Pure data: what a
//! setting does is the code of `mods/load/policy` and its neighbours, which read a name only from
//! the homes this table gives it.

use indexmap::IndexMap;
use serde::Serialize;

/// Where a name is read: the process `environment`, the `env` of the user's settings
/// (`user-env`), `managed` settings, the user's own settings (`user`), any settings file
/// (`any-file`), or `guard-option`, an entry under `pluginConfigs` for the built-in guard that only
/// managed settings can set.
pub const SETTING_HOMES: &[&str] = &[
    "environment",
    "user-env",
    "managed",
    "user",
    "any-file",
    "guard-option",
];

/// One setting's homes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSetting {
    /// Where the name is read.
    pub homes: Vec<&'static str>,
    /// True when the user's own settings count only on a machine with no managed settings, in a
    /// session whose credential is not a Team or Enterprise `/login` ("Where": "User settings only on a
    /// machine with no managed settings, in a session whose credential isn't a Team or Enterprise
    /// `/login`").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_only_without_policy: Option<bool>,
}

fn setting(homes: &[&'static str], user_only_without_policy: bool) -> ModSetting {
    ModSetting {
        homes: homes.to_vec(),
        user_only_without_policy: user_only_without_policy.then_some(true),
    }
}

/// The settings and environment variables that affect mods, by name.
#[must_use]
pub fn mod_settings() -> IndexMap<&'static str, ModSetting> {
    IndexMap::from([
        (
            "CLAUDE_CODE_PLUGIN_DIRS",
            setting(&["environment", "user-env"], false),
        ),
        (
            "CLAUDE_CODE_PLUGIN_DIR_WATCH",
            setting(&["environment"], false),
        ),
        ("prependPlugins", setting(&["managed", "user"], true)),
        ("appendPlugins", setting(&["managed", "user"], true)),
        ("allowManagedModsOnly", setting(&["guard-option"], false)),
        (
            "allowModsToOverrideDenyRules",
            setting(&["guard-option"], false),
        ),
        ("allowManagedHooksOnly", setting(&["managed"], false)),
        ("disableAllHooks", setting(&["any-file"], false)),
        ("disableSideloadFlags", setting(&["managed"], false)),
        ("pluginConfigs", setting(&["user", "managed"], false)),
    ])
}

/// The built-in guard's id in `pluginConfigs` (the only form there) and, as `prependPlugins` also accepts, its short id.
pub const GUARD_CONFIG_ID: &str = "cc-plugin-sec-default@builtin";
/// The built-in guard's short id, as `prependPlugins` accepts it.
pub const GUARD_ID: &str = "sec-default@builtin";
/// What `/plugin` and the debug log call the guard.
pub const GUARD_NAME: &str = "cc-plugin-sec-default";

crate::registry::exports! {
    "protocol/mods-settings" {
        "SETTING_HOMES" => ok || SETTING_HOMES;
        "MOD_SETTINGS" => ok mod_settings;
        "GUARD_CONFIG_ID" => ok || GUARD_CONFIG_ID;
        "GUARD_ID" => ok || GUARD_ID;
        "GUARD_NAME" => ok || GUARD_NAME;
    }
}
