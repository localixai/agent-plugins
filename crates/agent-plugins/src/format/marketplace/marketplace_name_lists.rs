//! Port of `src/format/marketplace/marketplace-name-lists.ts`, switched.
//!
//! The names the docs print (plugins/marketplace-reference.md, "Reserved names"): one list per
//! group, in the docs' order. The checks that use them are `marketplace-names.ts`.

use std::sync::LazyLock;

use indexmap::{IndexMap, IndexSet};

/// The official marketplace names, in the docs' order.
pub const OFFICIAL_NAMES: [&str; 14] = [
    "claude-code-marketplace",
    "claude-code-plugins",
    "claude-plugins-official",
    "anthropic-marketplace",
    "anthropic-plugins",
    "agent-skills",
    "anthropic-agent-skills",
    "life-sciences",
    "knowledge-work-plugins",
    "claude-for-legal",
    "claude-for-financial-services",
    "financial-services-plugins",
    "first-party-plugins",
    "claude-tag-plugins",
];
/// The official names whose auto-update default is off, though they are official: "on for
/// Anthropic's official marketplaces such as `claude-plugins-official`, off for
/// `knowledge-work-plugins` and `first-party-plugins`" (plugins/loading.md, "Which marketplaces and
/// plugins auto-update", item 3). The one list: the registry's plan and the update pass read it.
pub const OFFICIAL_BUT_OFF: [&str; 2] = ["knowledge-work-plugins", "first-party-plugins"];

/// Whether `name` is one of `OFFICIAL_BUT_OFF`.
#[must_use]
pub fn is_official_but_off(name: &str) -> bool {
    OFFICIAL_BUT_OFF.contains(&name)
}

/// The community marketplace names.
pub const COMMUNITY_NAMES: [&str; 3] =
    ["claude-community", "claude-plugins-community", "healthcare"];
/// The plugin directory names.
pub const DIRECTORY_NAMES: [&str; 2] = ["anthropic-plugin-directory", "claude-plugin-directory"];

/// The three lists whose names the `anthropics` organization on GitHub may use.
pub static OFFICIAL_LISTS: LazyLock<IndexSet<&'static str>> = LazyLock::new(|| {
    OFFICIAL_NAMES
        .iter()
        .chain(COMMUNITY_NAMES.iter())
        .chain(DIRECTORY_NAMES.iter())
        .copied()
        .collect()
});

/// Names Claude Code gives plugins that come from no marketplace, and what each one means.
pub static NON_MARKETPLACE_NAMES: LazyLock<IndexMap<&'static str, &'static str>> =
    LazyLock::new(|| {
        IndexMap::from([
            ("inline", "is reserved for --plugin-dir session plugins"),
            ("builtin", "is reserved for built-in plugins"),
            (
                "skills-dir",
                "is reserved for plugins auto-loaded from .claude/skills/",
            ),
            (
                "synced",
                "is reserved for plugins synced from your claude.ai account",
            ),
            (
                "claude-plugin-test",
                "is reserved for plugins loaded by claude plugin test",
            ),
        ])
    });

/// Reserved "in any casing" (requires Claude Code v2.1.275 or later).
pub static ANY_CASING_NAMES: LazyLock<IndexSet<&'static str>> =
    LazyLock::new(|| IndexSet::from(["npm", "pip", "uv", "cargo", "github", "gh"]));

/// Member names every JavaScript object has (requires Claude Code v2.1.296 or later).
pub static OBJECT_MEMBER_NAMES: LazyLock<IndexSet<&'static str>> = LazyLock::new(|| {
    IndexSet::from([
        "constructor",
        "hasOwnProperty",
        "isPrototypeOf",
        "propertyIsEnumerable",
        "toLocaleString",
        "toString",
        "valueOf",
    ])
});

crate::registry::exports! {
    "format/marketplace/marketplace-name-lists" {
        "OFFICIAL_NAMES" => ok || OFFICIAL_NAMES;
        "OFFICIAL_BUT_OFF" => ok || OFFICIAL_BUT_OFF;
        "isOfficialButOff" => ok |name: String| is_official_but_off(&name);
        "COMMUNITY_NAMES" => ok || COMMUNITY_NAMES;
        "DIRECTORY_NAMES" => ok || DIRECTORY_NAMES;
        "NON_MARKETPLACE_NAMES" => ok || crate::registry::Map { entries: NON_MARKETPLACE_NAMES.iter().collect::<Vec<_>>() };
        "ANY_CASING_NAMES" => ok || crate::registry::Set { members: ANY_CASING_NAMES.iter().collect::<Vec<_>>() };
        "OBJECT_MEMBER_NAMES" => ok || crate::registry::Set { members: OBJECT_MEMBER_NAMES.iter().collect::<Vec<_>>() };
    }
}
