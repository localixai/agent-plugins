//! Port of `src/host/catalog.ts`, switched.
//!
//! What one plugin's Markdown components read to: the formats' own readers, once, at load. The
//! catalog is kept with the core's copy of the plugin (`GivenPlugin`, `host/core_view.rs`), which
//! the edge makes once per `Plugin` object the host gives, so a host update that keeps a plugin
//! does not read its Markdown again.

use std::collections::HashSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::core_skills::core_skill_of;
use super::core_view::GivenPlugin;
use crate::format::agent::read_agent;
use crate::format::agent_record::{
    AgentOrigin, AgentReadResult, AgentRecord, PluginOfOrigin, PluginOrigin, PluginSource,
};
use crate::format::command::{
    CommandEntry, CommandEntryContext, CommandLayout, read_command, read_command_entry,
};
use crate::format::issues::PluginIssue;
use crate::format::output_style::{
    OutputStyle, OutputStyleContext, last_per_name, read_output_style,
};
use crate::format::skill::read_skill;
use crate::format::skill_name::{SkillLayout, SkillOrigin, self_named_command};
use crate::js::{JsObject, JsValue};
use crate::loading::readers_text::PluginTextFile;
use crate::registry::Own;
use crate::skills::command_entries::command_entry_grants;

/// What one plugin's Markdown components read to: the formats' own readers, once, at load.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCatalog {
    /// Its skills (`Skill`).
    pub skills: Vec<JsValue>,
    /// Its commands (`Command`).
    pub commands: Vec<JsValue>,
    /// Its agents.
    pub agents: Vec<AgentRecord>,
    /// Its output styles.
    pub output_styles: Vec<OutputStyle>,
    /// What the readers found.
    pub issues: Vec<PluginIssue>,
}

/// The parts of a `Plugin` the catalog reads.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginParts {
    #[serde(default)]
    name: String,
    #[serde(default)]
    skills: Vec<PluginTextFile>,
    #[serde(default)]
    commands: Vec<PluginTextFile>,
    #[serde(default)]
    agents: Vec<PluginTextFile>,
    #[serde(default)]
    output_styles: Vec<PluginTextFile>,
}

/// One component per name, the first one kept: Claude Code 2.1.319 registers two agent files that
/// declare the same `name` once (spec/judgments.md, "oracle 2.1.319"; which one it keeps is not
/// observable in the session's `init` event, so the kit keeps the first in loading order).
fn first_per_name<T>(items: Vec<T>, name_of: impl Fn(&T) -> String) -> Vec<T> {
    let mut seen = HashSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(name_of(item)))
        .collect()
}

/// `db:migrate` is the file `db/migrate.md` below its folder.
fn relative_path(file: &PluginTextFile) -> String {
    format!("{}.md", file.name.split(':').collect::<Vec<_>>().join("/"))
}

fn path_of(file: &PluginTextFile, folder: &str) -> String {
    file.path
        .clone()
        .unwrap_or_else(|| format!("{folder}/{}", relative_path(file)))
}

/// The plugin's own `SKILL.md` has no skill folder: its directory is the plugin root.
fn origin_of(plugin: &PluginParts, root_skill: bool) -> JsValue {
    let mut origin = JsObject::from([
        ("kind".to_owned(), JsValue::String("plugin".to_owned())),
        ("plugin".to_owned(), JsValue::String(plugin.name.clone())),
    ]);
    if root_skill {
        origin.insert("rootSkill".to_owned(), JsValue::Bool(true));
    }
    JsValue::Object(origin)
}

/// The `command` member of a read skill or command.
fn command_of(item: &JsValue) -> String {
    item.get("command")
        .and_then(JsValue::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn read_skills(plugin: &PluginParts, core: bool) -> (Vec<JsValue>, Vec<PluginIssue>) {
    if core {
        let skills = plugin
            .skills
            .iter()
            .map(|file| core_skill_of(&plugin.name, file));
        return (skills.collect(), Vec::new());
    }
    let reads: Vec<_> = plugin
        .skills
        .iter()
        .map(|file| {
            let dir_name = if file.alone == Some(true) {
                self_named_command(&file.name, &plugin.name)
            } else {
                file.name.clone()
            };
            let layout = SkillLayout {
                origin: SkillOrigin::Plugin {
                    plugin: plugin.name.clone(),
                },
                dir_name,
                fixed_name: Some(true),
            };
            let origin = origin_of(plugin, file.directory.as_deref() == Some("."));
            read_skill(&path_of(file, "skills"), &file.text, &layout, origin)
        })
        .collect();
    let issues = reads.iter().flat_map(|read| read.issues.clone()).collect();
    let skills = reads
        .into_iter()
        .map(|read| JsValue::Object(read.skill))
        .collect();
    (first_per_name(skills, command_of), issues)
}

/// A manifest command entry, read as `loadCommandEntries` reads it: the rules of `allowedTools` that
/// do not parse are dropped with a warning and grant nothing, and the command still loads.
fn read_entry(
    plugin: &PluginParts,
    file: &PluginTextFile,
    entry: CommandEntry,
) -> Result<(Option<JsValue>, Vec<PluginIssue>), String> {
    let grants = entry
        .allowed_tools
        .as_ref()
        .map(|rules| command_entry_grants(&file.name, rules));
    let source = entry.source.is_some().then_some(file.text.as_str());
    let checked = CommandEntry {
        allowed_tools: grants.as_ref().map(|grants| grants.rules.clone()),
        ..entry
    };
    let context = CommandEntryContext {
        origin: origin_of(plugin, false),
    };
    let read = read_command_entry(&file.name, &checked, source, &context)?;
    let mut issues = grants.map(|grants| grants.issues).unwrap_or_default();
    issues.extend(read.issues);
    Ok((read.command.map(JsValue::Object), issues))
}

fn read_commands(plugin: &PluginParts) -> Result<(Vec<JsValue>, Vec<PluginIssue>), String> {
    let mut commands = Vec::new();
    let mut issues = Vec::new();
    for file in &plugin.commands {
        let entry = (!file.entry.is_undefined())
            .then(|| CommandEntry::deserialize(file.entry.clone()).unwrap_or_default());
        let (command, found) = match entry {
            None => {
                let layout = CommandLayout {
                    origin: origin_of(plugin, false),
                    relative_path: relative_path(file),
                };
                let read = read_command(&path_of(file, "commands"), &file.text, &layout)?;
                (read.command.map(JsValue::Object), read.issues)
            }
            Some(entry) => read_entry(plugin, file, entry)?,
        };
        commands.extend(command);
        issues.extend(found);
    }
    Ok((first_per_name(commands, command_of), issues))
}

fn read_agents(plugin: &PluginParts) -> (Vec<AgentRecord>, Vec<PluginIssue>) {
    let mut agents = Vec::new();
    let mut issues = Vec::new();
    for file in &plugin.agents {
        let origin = AgentOrigin::Plugin(PluginOrigin {
            path: path_of(file, "agents"),
            source: PluginSource::Plugin,
            plugin: PluginOfOrigin {
                name: plugin.name.clone(),
                relative_path: Some(relative_path(file)),
            },
        });
        match read_agent(&file.text, &origin) {
            AgentReadResult::Loaded {
                agent,
                issues: found,
            } => {
                agents.push(*agent);
                issues.extend(found);
            }
            AgentReadResult::Skipped { issues: found, .. } => issues.extend(found),
        }
    }
    (
        first_per_name(agents, |agent| agent.scoped_name.clone()),
        issues,
    )
}

fn read_styles(plugin: &PluginParts) -> (Vec<OutputStyle>, Vec<PluginIssue>) {
    let context = OutputStyleContext {
        plugin: Some(plugin.name.clone()),
    };
    let reads: Vec<_> = plugin
        .output_styles
        .iter()
        .map(|file| read_output_style(&path_of(file, "output-styles"), &file.text, &context))
        .collect();
    let styles: Vec<OutputStyle> = reads.iter().map(|read| read.style.clone()).collect();
    let issues = reads.into_iter().flat_map(|read| read.issues).collect();
    (last_per_name(&styles), issues)
}

/// Reads the skills, commands, agents and output styles of a loaded plugin with the format readers.
/// Judgment (spec/judgments.md): the catalog is cached with the core's copy of the `Plugin` object,
/// which is immutable, so a host update that keeps a plugin does not read its Markdown again; a new
/// object, even for the same id, is read.
///
/// # Errors
///
/// When a reader refuses its origin (none does: the catalog writes them).
pub fn read_catalog(given: &GivenPlugin, markdown: &JsValue) -> Result<Arc<PluginCatalog>, String> {
    given.catalog(|| read_plugin(given, markdown))
}

/// The plugin's Markdown is `markdown` (its `skills`, `commands`, `agents` and `outputStyles`, which
/// the core's copy leaves out, `markdownOf` at the edge); a core package's view holds its own.
fn read_plugin(given: &GivenPlugin, markdown: &JsValue) -> Result<PluginCatalog, String> {
    let source = if markdown.is_undefined() {
        &given.plugin
    } else {
        markdown
    };
    let mut plugin = PluginParts::deserialize(source.clone()).unwrap_or_default();
    given.name().clone_into(&mut plugin.name);
    let (skills, skill_issues) = read_skills(&plugin, given.core.is_some());
    let (commands, command_issues) = read_commands(&plugin)?;
    let (agents, agent_issues) = read_agents(&plugin);
    let (output_styles, style_issues) = read_styles(&plugin);
    let mut issues = skill_issues;
    issues.extend(command_issues);
    issues.extend(agent_issues);
    issues.extend(style_issues);
    Ok(PluginCatalog {
        skills,
        commands,
        agents,
        output_styles,
        issues,
    })
}

crate::registry::exports! {
    "host/catalog" {
        "readCatalog" => fallible |given: Own<GivenPlugin>, markdown: JsValue|
            read_catalog(&given, &markdown).map(|catalog| catalog.as_ref().clone());
    }
}
