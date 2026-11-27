//! Port of `src/cli/tag-plan.ts`, switched.
//!
//! `claude plugin tag` (plugins/cli-reference.md § plugin tag): the release tag of a plugin, its
//! preview, and the git commands that create and push it, from what the host observed.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, blocks_loading};
use crate::format::manifest::validate_manifest;
use crate::js::JsValue;

/// The marketplace entry that lists the plugin.
#[derive(Clone, Debug, Deserialize)]
pub struct TagMarketplace {
    /// Its file.
    pub file: String,
    /// The version the entry declares.
    #[serde(default)]
    pub version: Option<String>,
}

/// Host observations, after walking upward for the marketplace that lists this directory.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagFacts {
    /// The plugin's directory.
    pub directory: String,
    /// Its `plugin.json`, as loaded.
    pub manifest: JsValue,
    /// What validating the folder reported.
    pub validation_issues: Vec<PluginIssue>,
    /// The marketplace entry, when one lists the plugin.
    #[serde(default)]
    pub marketplace: Option<TagMarketplace>,
    /// The working tree has changes.
    pub dirty: bool,
    /// The tag is already there.
    pub tag_exists: bool,
}

/// The flags of `claude plugin tag`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagOptions {
    /// `--dry-run`.
    #[serde(default)]
    pub dry_run: Option<bool>,
    /// `--force`.
    #[serde(default)]
    pub force: Option<bool>,
    /// `--message`.
    #[serde(default)]
    pub message: Option<String>,
    /// `--push`.
    #[serde(default)]
    pub push: Option<bool>,
    /// `--remote`.
    #[serde(default)]
    pub remote: Option<String>,
}

/// One git command, as exec arguments.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagCommand {
    /// Always `git`.
    pub command: String,
    /// The arguments.
    pub args: Vec<String>,
    /// Where it runs.
    pub cwd: String,
}

/// What `claude plugin tag` will do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagPlan {
    /// Nothing refuses the tag.
    pub ready: bool,
    /// `<name>--v<version>`.
    pub tag: String,
    /// The push command.
    pub push: TagCommand,
    /// The remote pushed to.
    pub remote: String,
    /// `--dry-run`.
    pub dry_run: bool,
    /// `--push`.
    pub pushes: bool,
    /// The lines shown before anything runs.
    pub preview: Vec<String>,
    /// The commands to run.
    pub commands: Vec<TagCommand>,
    /// Why the tag is refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
}

fn text_of<'a>(manifest: &'a JsValue, key: &str) -> Option<&'a str> {
    manifest.get(key).and_then(JsValue::as_str)
}

/// Judgment: labels and refusal wording are not specified; no undocumented text is a spec claim.
fn tag_refusal(facts: &TagFacts, force: bool) -> Option<&'static str> {
    let mut issues = validate_manifest(&facts.manifest);
    issues.extend(facts.validation_issues.iter().cloned());
    if blocks_loading(&issues) {
        return Some("Plugin validation failed");
    }
    let version = text_of(&facts.manifest, "version");
    if version.is_none_or(str::is_empty) {
        return Some("The plugin has no version");
    }
    if let Some(marketplace) = &facts.marketplace
        && marketplace.version.as_deref() != version
    {
        return Some("The plugin and marketplace entry must declare the same version");
    }
    if !force && facts.dirty {
        return Some("The plugin working tree is dirty");
    }
    (!force && facts.tag_exists).then_some("The release tag already exists")
}

/// Human preview is copyable shell text; execution still uses the original argument vector.
fn display_argument(value: &str) -> String {
    let plain = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_./:@%+=,-".contains(c));
    if plain {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn line_of(command: &TagCommand) -> String {
    let args: Vec<String> = command
        .args
        .iter()
        .map(|arg| display_argument(arg))
        .collect();
    args.join(" ")
}

fn git(args: Vec<String>, cwd: &str) -> TagCommand {
    TagCommand {
        command: "git".to_owned(),
        args,
        cwd: cwd.to_owned(),
    }
}

/// plugins/cli-reference.md § plugin tag. Commands are exec arguments, never interpolated shell text.
#[must_use]
pub fn plan_plugin_tag(options: &TagOptions, facts: &TagFacts) -> TagPlan {
    let name = text_of(&facts.manifest, "name").unwrap_or("undefined");
    let version = text_of(&facts.manifest, "version").unwrap_or("");
    let tag = format!("{name}--v{version}");
    let remote = options
        .remote
        .clone()
        .unwrap_or_else(|| "origin".to_owned());
    let force = options.force == Some(true);
    let annotation = options
        .message
        .clone()
        .unwrap_or_else(|| format!("{name} {version}"))
        .replace("%s", version);
    let mut create_args = vec!["tag".to_owned(), "-a".to_owned()];
    if force {
        create_args.push("-f".to_owned());
    }
    create_args.extend([tag.clone(), "-m".to_owned(), annotation]);
    let create = git(create_args, &facts.directory);
    let push = git(
        vec!["push".to_owned(), remote.clone(), tag.clone()],
        &facts.directory,
    );
    let refusal = tag_refusal(facts, force).map(str::to_owned);
    let dry_run = options.dry_run == Some(true);
    let pushes = options.push == Some(true);
    let mut preview = vec![
        format!("Plugin: {name}"),
        format!(
            "Version: {version} ({}/.claude-plugin/plugin.json)",
            facts.directory
        ),
    ];
    if let Some(marketplace) = &facts.marketplace {
        preview.push(format!("Marketplace entry: {}", marketplace.file));
    }
    preview.extend([
        format!("Tag: {tag}"),
        format!("git {}", line_of(&create)),
        format!("git {}", line_of(&push)),
    ]);
    let commands = if refusal.is_some() || dry_run {
        Vec::new()
    } else if pushes {
        vec![create, push.clone()]
    } else {
        vec![create]
    };
    TagPlan {
        ready: refusal.is_none(),
        tag,
        push,
        remote,
        dry_run,
        pushes,
        preview,
        commands,
        refusal,
    }
}

/// What running the plan's commands did.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct TagEffects {
    /// The tag was created.
    pub created: bool,
    /// The tag was pushed.
    #[serde(default)]
    pub pushed: Option<bool>,
    /// What failed.
    #[serde(default)]
    pub error: Option<String>,
}

/// How `claude plugin tag` ends.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagFinish {
    /// 0 or 1.
    pub exit_code: u8,
    /// The tag exists locally now.
    pub tag_created: bool,
    /// The lines printed.
    pub lines: Vec<String>,
}

fn finish(exit_code: u8, tag_created: bool, lines: Vec<String>) -> TagFinish {
    TagFinish {
        exit_code,
        tag_created,
        lines,
    }
}

/// A failed push does not undo the tag already created locally.
#[must_use]
pub fn finish_plugin_tag(plan: &TagPlan, effects: &TagEffects) -> TagFinish {
    if !plan.ready {
        let line = plan
            .refusal
            .clone()
            .unwrap_or_else(|| "Cannot tag this plugin".to_owned());
        return finish(1, false, vec![line]);
    }
    if plan.dry_run {
        return finish(0, false, plan.preview.clone());
    }
    if !effects.created {
        let line = effects
            .error
            .clone()
            .unwrap_or_else(|| "Tag creation failed".to_owned());
        return finish(1, false, vec![line]);
    }
    let created = format!("Created tag {}", plan.tag);
    if !plan.pushes {
        return finish(
            0,
            true,
            vec![created, format!("git {}", line_of(&plan.push))],
        );
    }
    if effects.pushed == Some(true) {
        finish(0, true, vec![created, format!("Pushed to {}", plan.remote)])
    } else {
        let line = effects
            .error
            .clone()
            .unwrap_or_else(|| "Tag push failed".to_owned());
        finish(1, true, vec![created, line])
    }
}

crate::registry::exports! {
    "cli/tag-plan" {
        "planPluginTag" => ok |options: TagOptions, facts: TagFacts| plan_plugin_tag(&options, &facts);
        "finishPluginTag" => ok |plan: TagPlan, effects: TagEffects| finish_plugin_tag(&plan, &effects);
    }
}

#[cfg(test)]
mod tests {
    use super::display_argument;

    #[test]
    fn quotes_an_argument_a_shell_would_split() {
        assert_eq!(
            display_argument("a-b_c.d/e:f@g%h+i=j,k"),
            "a-b_c.d/e:f@g%h+i=j,k"
        );
        assert_eq!(display_argument("it's"), "'it'\\''s'");
        assert_eq!(display_argument(""), "''");
    }
}
