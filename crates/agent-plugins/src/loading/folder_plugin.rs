//! Port of `src/loading/folder-plugin.ts`, switched.
//!
//! Whether a folder is a plugin (plugins/loading.md, "Find where a plugin came from"; plugins/
//! manifest-reference.md, "Standard layout"; plugins/create.md, "Plugin layout" and "Name the
//! plugin's skills"; skills.md, "Skill folder as a plugin"): the probe a host runs over a folder of
//! a skills directory (`@skills-dir`) or one handed to `--plugin-dir` (`@inline`). It reads the
//! folder through `PluginFiles` and decides; loading the plugin stays `loadPlugin`'s.

use serde::{Deserialize, Serialize};

use super::load_plugin::load_manifest;
use super::readers_text::PluginTextFile;
use crate::format::issues::PluginIssue;
use crate::format::layout::{ComponentKind, components_at};
use crate::format::listing::list_sorted;
use crate::format::load_files::Files;
use crate::format::names::name_load_issues;
use crate::format::skill::read_skill;
use crate::format::skill_name::{SkillLayout, SkillOrigin};
use crate::js::{JsObject, JsValue};
use crate::ports::Signal;

/// `skills-dir`: a folder of `~/.claude/skills/` or `.claude/skills/`; `inline`: a `--plugin-dir`
/// directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FolderOrigin {
    /// A folder of a skills directory.
    #[serde(rename = "skills-dir")]
    SkillsDir,
    /// A `--plugin-dir` directory.
    #[serde(rename = "inline")]
    Inline,
}

/// `FOLDER_ORIGINS`.
pub const FOLDER_ORIGINS: [FolderOrigin; 2] = [FolderOrigin::SkillsDir, FolderOrigin::Inline];

struct OriginRules {
    /// Where a marketplace name would stand in `<name>@<origin>`.
    suffix: &'static str,
    /// Without `.claude-plugin/plugin.json` the folder is not a plugin of this origin.
    manifest_required: bool,
}

/// `@skills-dir` is "a plugin directory that has a `.claude-plugin/plugin.json`"; a plugin loaded
/// with `--plugin-dir` may omit it ("Claude Code names the plugin after its directory").
const fn origins(origin: FolderOrigin) -> OriginRules {
    match origin {
        FolderOrigin::SkillsDir => OriginRules {
            suffix: "skills-dir",
            manifest_required: true,
        },
        FolderOrigin::Inline => OriginRules {
            suffix: "inline",
            manifest_required: false,
        },
    }
}

/// A folder that is a plugin (`FolderPlugin`).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderPlugin {
    /// The manifest's `name`, else the folder's name.
    pub name: String,
    /// `<name>@skills-dir` or `<name>@inline`.
    pub id: String,
    /// The manifest's `defaultEnabled`, `true` when it sets none.
    pub default_enabled: bool,
    /// A `SKILL.md` sits at the plugin root.
    pub root_skill: bool,
    /// The component kinds the folder shows at its root, in layout order.
    pub components: Vec<ComponentKind>,
}

/// A plain skill named after the folder.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PlainSkill {
    /// The folder's name.
    pub name: String,
}

/// What a probe found (`FolderProbe`).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderProbe {
    /// The plugin, when the folder is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<FolderPlugin>,
    /// A skills-directory folder with a root `SKILL.md` and no manifest: a plain skill named after
    /// the folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plain_skill: Option<PlainSkill>,
    /// Why a manifest or a name keeps the folder from being a plugin.
    pub issues: Vec<PluginIssue>,
}

/// What the folder shows (`FolderFacts`).
struct FolderFacts {
    root_skill: bool,
    components: Vec<ComponentKind>,
}

fn plugin_of(
    name: &str,
    origin: FolderOrigin,
    default_enabled: bool,
    facts: FolderFacts,
) -> FolderPlugin {
    FolderPlugin {
        name: name.to_owned(),
        id: format!("{name}@{}", origins(origin).suffix),
        default_enabled,
        root_skill: facts.root_skill,
        components: facts.components,
    }
}

/// A folder without a manifest: a `--plugin-dir` plugin named after the folder, a plain skill, or
/// nothing.
fn probe_unnamed(folder_name: &str, origin: FolderOrigin, facts: FolderFacts) -> FolderProbe {
    if origins(origin).manifest_required {
        let plain_skill = facts.root_skill.then(|| PlainSkill {
            name: folder_name.to_owned(),
        });
        return FolderProbe {
            plain_skill,
            ..FolderProbe::default()
        };
    }
    if !facts.root_skill && facts.components.is_empty() {
        return FolderProbe::default();
    }
    let issues = name_load_issues(folder_name);
    if !issues.is_empty() {
        return FolderProbe {
            issues,
            ..FolderProbe::default()
        };
    }
    FolderProbe {
        plugin: Some(plugin_of(folder_name, origin, true, facts)),
        ..FolderProbe::default()
    }
}

/// The options of a probe.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ProbeOptions {
    /// Aborts the reads.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// Probes one folder. With a manifest the folder is a plugin named by it. Without one, a
/// `--plugin-dir` folder with any component (or a root `SKILL.md`) is a plugin named after the
/// folder, and a skills-directory folder is a plain skill when it has a root `SKILL.md`, otherwise
/// nothing.
pub async fn probe_folder(
    files: &Files,
    folder_name: &str,
    origin: FolderOrigin,
    options: &ProbeOptions,
) -> FolderProbe {
    let signal = options.signal.as_ref();
    let listed = list_sorted(files, ".", signal).await.unwrap_or_default();
    let names: Vec<String> = listed.into_iter().map(|entry| entry.name).collect();
    let facts = FolderFacts {
        root_skill: names.iter().any(|name| name == "SKILL.md"),
        components: components_at(&names),
    };
    let step = load_manifest(files, signal).await;
    if !step.present {
        return probe_unnamed(folder_name, origin, facts);
    }
    let manifest = &step.manifest;
    if manifest.is_undefined() {
        return FolderProbe {
            issues: step.issues,
            ..FolderProbe::default()
        };
    }
    let name = manifest
        .get("name")
        .and_then(JsValue::as_str)
        .unwrap_or_default();
    let default_enabled = manifest.get("defaultEnabled").and_then(JsValue::as_bool);
    FolderProbe {
        plugin: Some(plugin_of(
            name,
            origin,
            default_enabled.unwrap_or(true),
            facts,
        )),
        plain_skill: None,
        issues: step.issues,
    }
}

/// Whether a skills-directory plugin is on: "The manifest's `defaultEnabled`, unless a settings
/// file sets `<name>@skills-dir` to `true` or `false`."
#[must_use]
pub fn skills_dir_plugin_enabled(default_enabled: bool, setting: Option<bool>) -> bool {
    setting.unwrap_or(default_enabled)
}

/// What `skillsDirCommands` reads of a plugin: its name and skills.
#[derive(Clone, Debug, Deserialize)]
pub struct SkillsDirPlugin {
    /// The plugin's name.
    pub name: String,
    /// Its skill files.
    pub skills: Vec<PluginTextFile>,
}

/// Whose skills directory a folder is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillsDirScope {
    /// `~/.claude/skills/`.
    User,
    /// `.claude/skills/`.
    Project,
}

/// A folder of a skills directory (`SkillsDirFolder`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsDirFolder {
    /// The folder's name under the skills directory.
    pub name: String,
    /// `user` or `project`.
    pub scope: SkillsDirScope,
    /// The text of the `SKILL.md` at the folder's root, when there is one.
    #[serde(default)]
    pub root_skill: Option<String>,
}

/// `/<command>` of a `SKILL.md` read with `layout`.
fn command_of(path: &str, text: &str, layout: &SkillLayout, origin: JsValue) -> String {
    let skill = read_skill(path, text, layout, origin).skill;
    let command = skill
        .get("command")
        .and_then(JsValue::as_str)
        .unwrap_or_default();
    format!("/{command}")
}

/// The commands of a skills-directory plugin (plugins/create.md, "Name the plugin's skills"): "The
/// root skill ... is also a personal skill, so you invoke it as `/my-tool`, not
/// `/my-tool:my-tool`. Skills you add under `skills/` inside the plugin get the plugin-name
/// prefix, such as `/my-tool:example`." The root skill is the folder's, not the plugin's: it is a
/// skill of the skills directory whether or not the plugin loads it too, so the plugin's own copy
/// of it is left out.
#[must_use]
pub fn skills_dir_commands(plugin: &SkillsDirPlugin, folder: &SkillsDirFolder) -> Vec<String> {
    let own = folder
        .root_skill
        .iter()
        .map(|text| root_command(folder, text));
    let origin = JsValue::Object(JsObject::from_iter([
        ("kind".to_owned(), JsValue::from("plugin")),
        ("plugin".to_owned(), JsValue::from(plugin.name.as_str())),
    ]));
    let prefixed = plugin
        .skills
        .iter()
        .filter(|file| file.directory.as_deref() != Some("."))
        .map(|file| {
            let layout = SkillLayout {
                origin: SkillOrigin::Plugin {
                    plugin: plugin.name.clone(),
                },
                dir_name: file.name.clone(),
                fixed_name: None,
            };
            command_of(
                file.path.as_deref().unwrap_or("SKILL.md"),
                &file.text,
                &layout,
                origin.clone(),
            )
        });
    own.chain(prefixed).collect()
}

fn root_command(folder: &SkillsDirFolder, text: &str) -> String {
    let (kind, scope) = match folder.scope {
        SkillsDirScope::User => ("user", SkillOrigin::User),
        SkillsDirScope::Project => ("project", SkillOrigin::Project),
    };
    let origin = JsValue::Object(JsObject::from_iter([(
        "kind".to_owned(),
        JsValue::from(kind),
    )]));
    let layout = SkillLayout {
        origin: scope,
        dir_name: folder.name.clone(),
        fixed_name: None,
    };
    command_of("SKILL.md", text, &layout, origin)
}

crate::registry::exports! {
    "loading/folder-plugin" {
        "FOLDER_ORIGINS" => ok || FOLDER_ORIGINS;
        "probeFolder" => async ok |files: Files, folder_name: String, origin: FolderOrigin, options: Option<ProbeOptions>|
            probe_folder(&files, &folder_name, origin, &options.unwrap_or_default()).await;
        "skillsDirPluginEnabled" => ok |default_enabled: bool, setting: Option<bool>|
            skills_dir_plugin_enabled(default_enabled, setting);
        "skillsDirCommands" => ok |plugin: SkillsDirPlugin, folder: SkillsDirFolder|
            skills_dir_commands(&plugin, &folder);
    }
}
