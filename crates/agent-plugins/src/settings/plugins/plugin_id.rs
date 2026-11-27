//! Port of `src/settings/plugins/plugin-id.ts`, switched.
//!
//! A plugin id is `<name>@<origin>`: the origin is a marketplace name or one of the reserved ones
//! (`inline`, `skills-dir`, `synced`, plugins/loading.md, "Find where a plugin came from") or
//! `builtin` for a plugin inside Claude Code (settings-reference.md, `pluginConfigs`).

use serde::Serialize;

use crate::js::is_whitespace;

/// A name and the origin it comes from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ParsedPluginId {
    /// The plugin's name.
    pub name: String,
    /// A marketplace name or a reserved origin.
    pub origin: String,
}

/// One part of an id: not empty, no `@`, no whitespace (`[^@\s]+`).
fn is_part(text: &str) -> bool {
    !text.is_empty()
        && !text
            .chars()
            .any(|character| character == '@' || is_whitespace(character))
}

/// One `@`, no whitespace, a name and an origin on each side: the shape `enabledPlugins` reads
/// (`^([^@\s]+)@([^@\s]+)$`).
#[must_use]
pub fn parse_plugin_id(text: &str) -> Option<ParsedPluginId> {
    let (name, origin) = text.split_once('@')?;
    (is_part(name) && is_part(origin)).then(|| ParsedPluginId {
        name: name.to_owned(),
        origin: origin.to_owned(),
    })
}

/// Whether the text is a plugin id.
#[must_use]
pub fn is_plugin_id(text: &str) -> bool {
    parse_plugin_id(text).is_some()
}

/// A built-in plugin that two releases name differently, current id first. The docs give two:
/// `pluginConfigs` reads the Project instructions plugin under either id ("Later versions read an
/// entry under either ID"), and `prependPlugins` accepts the guard as `sec-default@builtin`, the id
/// every release reads, or as `cc-plugin-sec-default@builtin`.
pub type IdForms = [&'static str; 2];

/// The Project instructions plugin's two ids.
pub const AGENTS_MD_ID_FORMS: IdForms = ["cc-plugin-agents-md@builtin", "agents-md@builtin"];
/// The guard plugin's two ids.
pub const GUARD_ID_FORMS: IdForms = ["sec-default@builtin", "cc-plugin-sec-default@builtin"];

/// Every spelling of `id` among `registry`'s plugins, the current one first; `id` alone otherwise.
#[must_use]
pub fn spellings_of(registry: &[[String; 2]], id: &str) -> Vec<String> {
    registry
        .iter()
        .find(|forms| forms.iter().any(|form| form == id))
        .map_or_else(|| vec![id.to_owned()], |forms| forms.to_vec())
}

/// Whether the id is one of the guard plugin's spellings.
#[must_use]
pub fn is_guard_plugin_id(id: &str) -> bool {
    GUARD_ID_FORMS.contains(&id)
}

crate::registry::exports! {
    "settings/plugins/plugin-id" {
        "parsePluginId" => ok |text: String| parse_plugin_id(&text);
        "isPluginId" => ok |text: String| is_plugin_id(&text);
        "agentsMdIdForms" => ok || AGENTS_MD_ID_FORMS;
        "guardIdForms" => ok || GUARD_ID_FORMS;
        "spellingsOf" => ok |registry: Vec<[String; 2]>, id: String| spellings_of(&registry, &id);
        "isGuardPluginId" => ok |id: String| is_guard_plugin_id(&id);
    }
}
