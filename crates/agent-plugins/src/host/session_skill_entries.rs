//! Port of `src/host/session-skill-entries.ts`, switched.
//!
//! The session's skills and commands without their bodies, with every name that invokes them. The
//! host's state is its object (`HostState`, kept at the edge until it is the core's): its wrapper
//! passes each active plugin's id and catalog, and each plugin entry names its plugin by its index
//! into the state's `active` list, which the wrapper maps back to the host's `ActivePlugin`. The
//! body known at load stays behind the core's reader (`KnownBody`).

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::format::skill_types::{BodyReader, KnownBody};
use crate::js::{JsObject, JsValue, code_unit_length, slice_code_units};
use crate::ports::locations::Locations;
use crate::registry::Port;
use crate::skills::variables::skill_path_in;

/// Where a skill lives, as the host writes paths: absolute, each absent when the skill has none.
#[derive(Clone, Debug, Default, Serialize)]
pub struct SkillPlace {
    /// The `SKILL.md` or the command file. An inline manifest command has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// The skill's folder, the value of `${CLAUDE_SKILL_DIR}`. A command has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
}

fn text<'a>(invocable: &'a JsObject, key: &str) -> Option<&'a str> {
    invocable.get(key).and_then(JsValue::as_str)
}

/// A skill's place as the scan or the host wrote it, already absolute (`placeOf` without `below`).
#[must_use]
pub fn place_of(invocable: &JsObject) -> SkillPlace {
    SkillPlace {
        file: text(invocable, "path").map(str::to_owned),
        directory: invocable.get("dir").map(crate::js::string_of),
    }
}

/// The plugin's root, asked of the port once per plugin, when its first path needs it.
struct Root<'a> {
    plugin_id: &'a str,
    root: Option<String>,
}

impl Root<'_> {
    async fn get(&mut self, locations: &Port) -> String {
        if let Some(root) = &self.root {
            return root.clone();
        }
        let root = locations.plugin_root(self.plugin_id).await;
        self.root = Some(root.clone());
        root
    }
}

/// A plugin skill's place: its path and folder below the plugin's root (`skillPathIn`, the rule of
/// `${CLAUDE_SKILL_DIR}`).
async fn place_below(invocable: &JsObject, root: &mut Root<'_>, locations: &Port) -> SkillPlace {
    let mut place = SkillPlace::default();
    if let Some(path) = text(invocable, "path") {
        let base = root.get(locations).await;
        place.file = Some(skill_path_in(&base, path, locations).await);
    }
    if let Some(dir) = invocable.get("dir") {
        let base = root.get(locations).await;
        place.directory = Some(skill_path_in(&base, &crate::js::string_of(dir), locations).await);
    }
    place
}

fn folder_name(dir: &str) -> &str {
    let at = dir.rfind(['/', '\\']).map_or(0, |at| at + 1);
    dir.get(at..).unwrap_or_default()
}

/// The name a plugin skill's frontmatter `name` also gives it without the plugin prefix: "The bare
/// `/fancy` also invokes the skill unless another command already uses that name" (skills.md, "How a
/// skill gets its command name"). A skill that keeps its folder's name has no bare name (judgment: the
/// docs state it for a `name`), and neither has a plugin's root `SKILL.md`. A skill its folder named
/// despite a `name` takes the bare part of its `declaredCommand` (oracle 2.1.322).
fn bare_name(invocable: &JsObject) -> Option<String> {
    let origin = invocable.get("origin")?;
    let plugin = origin.get("plugin")?;
    if matches!(origin.get("rootSkill"), Some(JsValue::Bool(true))) {
        return None;
    }
    let dir = crate::js::string_of(invocable.get("dir")?);
    let named = text(invocable, "declaredCommand").or_else(|| text(invocable, "command"))?;
    let prefix = format!("{}:", crate::js::string_of(plugin));
    let bare = slice_code_units(named, code_unit_length(&prefix), None);
    (bare != folder_name(&dir)).then_some(bare)
}

/// The listed command, then the command a frontmatter `name` gives (`Skill.declaredCommand`).
fn commands_of(invocable: &JsObject) -> Vec<String> {
    let command = text(invocable, "command").unwrap_or_default().to_owned();
    match text(invocable, "declaredCommand") {
        Some(declared) => vec![command, declared.to_owned()],
        None => vec![command],
    }
}

/// What this reads of an active plugin: its id and its catalog's skills and commands, with bodies.
#[derive(Clone, Debug, Deserialize)]
pub struct PluginEntries {
    /// The plugin's id.
    pub id: String,
    /// `catalog.skills`.
    pub skills: Vec<JsObject>,
    /// `catalog.commands`.
    pub commands: Vec<JsObject>,
}

/// One plugin skill or command: an `Entry` whose plugin is its index into the state's `active`.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEntry {
    /// The index of its plugin in `active`.
    pub plugin: usize,
    /// The skill or command without its body.
    pub invocable: JsObject,
    /// Every name that invokes it, the listed command first.
    pub names: Vec<String>,
    /// The body known at load.
    pub read_body: BodyReader,
    /// `{ kind: 'plugin', pluginId }`.
    pub source: JsValue,
    /// Where it lives.
    #[serde(flatten)]
    pub place: SkillPlace,
}

fn source_of(plugin_id: &str) -> JsValue {
    let mut source = JsObject::new();
    source.insert("kind".to_owned(), "plugin".into());
    source.insert("pluginId".to_owned(), plugin_id.into());
    JsValue::Object(source)
}

/// The entries of one plugin. Oracle 2.1.319: a command, not a skill, wins a plugin-local name
/// collision.
async fn plugin_entries(index: usize, plugin: PluginEntries, locations: &Port) -> Vec<PluginEntry> {
    let PluginEntries {
        id,
        skills,
        commands,
    } = plugin;
    let taken: Vec<&str> = commands
        .iter()
        .filter_map(|command| text(command, "command"))
        .collect();
    let skills = skills
        .iter()
        .filter(|skill| !text(skill, "command").is_some_and(|command| taken.contains(&command)));
    let mut root = Root {
        plugin_id: &id,
        root: None,
    };
    let mut entries = Vec::new();
    for item in skills.chain(commands.iter()) {
        let mut invocable = item.clone();
        // The body stays behind `readBody`: the entry holds the head, as a folder entry does.
        let body = invocable
            .shift_remove("body")
            .map(|body| crate::js::string_of(&body));
        let place = place_below(&invocable, &mut root, locations).await;
        entries.push(PluginEntry {
            plugin: index,
            names: commands_of(&invocable),
            read_body: BodyReader::core(KnownBody(body.unwrap_or_default())),
            source: source_of(&id),
            place,
            invocable,
        });
    }
    entries
}

/// The plugins' skills and commands of one host generation: the plugin root of each is asked of the
/// state's `Locations` here, so a catalog read after it calls no port.
pub async fn entries_of(active: Vec<PluginEntries>, locations: &Port) -> Vec<PluginEntry> {
    let mut entries = Vec::new();
    for (index, plugin) in active.into_iter().enumerate() {
        entries.extend(plugin_entries(index, plugin, locations).await);
    }
    let mut used: HashSet<String> = entries
        .iter()
        .filter_map(|entry| text(&entry.invocable, "command").map(str::to_owned))
        .collect();
    for entry in &mut entries {
        let Some(bare) = bare_name(&entry.invocable) else {
            continue;
        };
        if used.insert(bare.clone()) {
            entry.names.push(bare);
        }
    }
    entries
}

crate::registry::exports! {
    "host/session-skill-entries" {
        "entriesOf" => async ok |active: Vec<PluginEntries>, locations: Port| entries_of(active, &locations).await;
    }
}
