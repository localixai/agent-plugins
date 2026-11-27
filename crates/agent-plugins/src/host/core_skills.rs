//! Port of `src/host/core-skills.ts`, switched.
//!
//! A core package's skill as a plugin skill (Agent Plugins Extended §18.1.5 item 5, J18.14): the
//! command `/<package>:<directory>`, listed and invocable by the model and the user. Only the six
//! Agent Skills fields have an effect (`name`, `description`, `license`, `compatibility`, `metadata`,
//! `allowed-tools`): every Claude Code field the file may also carry (`model`, `hooks`, `context`,
//! `disable-model-invocation`, `paths`, `shell`, `arguments`...) takes its default, so a core skill
//! acts the same in every core client. The loader already checked the file against Agent Skills
//! (core §7.1), so the Claude Code reader's findings on other fields are not the package's issues.
//! A member the read skill lacks is left out, as the boundary drops an `undefined` member.

use crate::format::skill::read_skill;
use crate::format::skill_name::{SkillLayout, SkillOrigin};
use crate::js::{JsObject, JsValue};
use crate::loading::readers_text::PluginTextFile;

/// What every field outside Agent Skills is in a core skill: Claude Code's value for an unset field.
fn claude_code_defaults() -> [(&'static str, JsValue); 7] {
    [
        ("arguments", JsValue::Array(Vec::new())),
        ("disableModelInvocation", JsValue::Bool(false)),
        ("userInvocable", JsValue::Bool(true)),
        ("disallowedTools", JsValue::Array(Vec::new())),
        ("background", JsValue::Bool(false)),
        ("paths", JsValue::Array(Vec::new())),
        ("shell", JsValue::String("bash".to_owned())),
    ]
}

/// The members a core skill takes from the read one as they are.
const TAKEN: [&str; 8] = [
    "name",
    "command",
    "path",
    "dir",
    "origin",
    "body",
    "frontmatter",
    "allowedTools",
];

/// The Agent Skills fields a core skill carries only when the file sets them.
const WHEN_SET: [&str; 4] = ["description", "metadata", "license", "compatibility"];

/// A core package's skill file as the plugin skill it loads as.
#[must_use]
pub fn core_skill_of(plugin_name: &str, file: &PluginTextFile) -> JsValue {
    let path = file
        .path
        .clone()
        .unwrap_or_else(|| format!("skills/{}/SKILL.md", file.name));
    let origin = JsValue::Object(JsObject::from([
        ("kind".to_owned(), JsValue::String("plugin".to_owned())),
        ("plugin".to_owned(), JsValue::String(plugin_name.to_owned())),
        ("core".to_owned(), JsValue::Bool(true)),
    ]));
    let layout = SkillLayout {
        origin: SkillOrigin::Plugin {
            plugin: plugin_name.to_owned(),
        },
        dir_name: file.name.clone(),
        fixed_name: Some(true),
    };
    let skill = read_skill(&path, &file.text, &layout, origin).skill;
    let member = |key: &str| skill.get(key).cloned().unwrap_or_default();
    let mut core = JsObject::new();
    core.insert("kind".to_owned(), JsValue::String("skill".to_owned()));
    for key in TAKEN {
        let value = member(key);
        if !value.is_undefined() {
            core.insert(key.to_owned(), value);
        }
    }
    for (key, value) in claude_code_defaults() {
        core.insert(key.to_owned(), value);
    }
    for key in WHEN_SET {
        let value = member(key);
        if !value.is_undefined() {
            core.insert(key.to_owned(), value);
        }
    }
    JsValue::Object(core)
}

crate::registry::exports! {
    "host/core-skills" {
        "coreSkillOf" => ok |plugin_name: String, file: PluginTextFile| core_skill_of(&plugin_name, &file);
    }
}
