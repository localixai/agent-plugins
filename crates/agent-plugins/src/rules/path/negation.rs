//! Port of `src/rules/path/negation.ts`, switched.
//!
//! `!` negation (permissions.md, "Special characters and `!` negation"): "A deny or ask pattern
//! that starts with `!` is a gitignore negation. It carves the paths it matches out of the `path`
//! or `./path` rules listed before it." The carve-out reaches only rules of the same source, and
//! only deny and ask rules carve (an allow rule that starts with `!` approves nothing).

use crate::rules::path::anchor::PathSpecifier;
use crate::rules::path::file_rules::{match_path_rule, path_specifier_of};
use crate::rules::path::tools::requested_path;
use crate::rules::restricts::restricts;
use crate::rules::types::{MatchContext, PermissionRule, PreparedCall, RuleSource};

/// What makes two rules come from one settings file or flag: a `match` over the kinds.
fn identity(source: &RuleSource) -> String {
    match source {
        RuleSource::Managed => "managed".to_owned(),
        RuleSource::EmbedderManaged => "embedder-managed".to_owned(),
        RuleSource::Project => "project".to_owned(),
        RuleSource::Local => "local".to_owned(),
        RuleSource::User => "user".to_owned(),
        RuleSource::FlagSettings { file } => format!("flag-settings:{file}"),
        RuleSource::Cli => "cli".to_owned(),
        RuleSource::Session => "session".to_owned(),
    }
}

/// The `!` rules after `rule` in the list that carve out of it: same source, behaviour and tool.
fn carve_outs_after(rule: &PermissionRule, later: &[PermissionRule]) -> Vec<PathSpecifier> {
    let mut found = Vec::new();
    for other in later {
        let specifier = path_specifier_of(other);
        let same_list = other.behavior == rule.behavior
            && other.value.tool_name == rule.value.tool_name
            && identity(&other.source) == identity(&rule.source);
        if let Some(specifier) = specifier.filter(|specifier| same_list && specifier.negated) {
            found.push(specifier);
        }
    }
    found
}

/// Whether `rules[index]`, which matched `call`, is carved out of by a later `!` rule of its own
/// source (so it covers the call no longer).
pub fn is_carved_out(
    rules: &[PermissionRule],
    index: usize,
    call: &PreparedCall,
    ctx: &MatchContext,
) -> Result<bool, String> {
    // Only deny and ask rules carve; an allow rule that starts with `!` approves nothing.
    let Some(rule) = rules.get(index).filter(|rule| restricts(rule.behavior)) else {
        return Ok(false);
    };
    // A call with no path the kit reads (LSP without `filePath`) is never carved out: there is
    // nothing for a `!` to match.
    if requested_path(&call.call, &ctx.paths).is_none() {
        return Ok(false);
    }
    let carve_outs = carve_outs_after(rule, rules.get(index + 1..).unwrap_or_default());
    Ok(!carve_outs.is_empty() && !match_path_rule(rule, call, ctx, &carve_outs)?)
}

crate::registry::exports! {
    "rules/path/negation" {
        "isCarvedOut" => fallible |rules: Vec<PermissionRule>, index: usize, call: PreparedCall, ctx: MatchContext| is_carved_out(&rules, index, &call, &ctx);
    }
}
