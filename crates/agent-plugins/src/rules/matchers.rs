//! Port of `src/rules/matchers.ts`, switched.

use indexmap::IndexMap;

use crate::registry::Function;
use crate::rules::bash::bash::match_bash_rule;
use crate::rules::bash::powershell::match_power_shell_rule;
use crate::rules::path::file_rules::match_file_rule;
use crate::rules::restricts::restricts;
use crate::rules::target::agent::match_agent_rule;
use crate::rules::target::domain::match_web_fetch_rule;
use crate::rules::target::skill::match_skill_rule;
use crate::rules::tools::rule_formats::RuleFormatId;
use crate::rules::types::{MatchContext, PermissionRule, PreparedCall};

/// Matches the text or domain specifier of one rule against one call. A rule reaches its matcher
/// only after its tool name matched and it is neither bare nor a parameter rule, so the matcher
/// reads `rule.specifier` and the call's input, and decides by `rule.behavior` where the docs
/// differ (an allow rule needs every Bash subcommand covered, a deny or ask rule any). The `Err`
/// is what the matcher throws (a path pattern too large for a regular expression).
pub type SpecifierMatcher =
    fn(&PermissionRule, &PreparedCall, &MatchContext) -> Result<bool, String>;

/// What an unwritten matcher answers: it fails on the safe side. An allow rule that matches
/// nothing approves nothing; a deny or ask rule covers every call of its tool until the real
/// matcher exists.
#[expect(
    clippy::unnecessary_wraps,
    reason = "an entry of the table, so it has the signature every matcher has"
)]
fn fails_closed(
    rule: &PermissionRule,
    _call: &PreparedCall,
    _ctx: &MatchContext,
) -> Result<bool, String> {
    Ok(restricts(rule.behavior))
}

/// `SPECIFIER_MATCHERS[format]`.
#[must_use]
pub fn specifier_matcher(format: RuleFormatId) -> SpecifierMatcher {
    match format {
        RuleFormatId::Command => |rule, call, ctx| Ok(match_bash_rule(rule, call, ctx)),
        RuleFormatId::PowershellCommand => {
            |rule, call, ctx| Ok(match_power_shell_rule(rule, call, ctx))
        }
        RuleFormatId::ReadPath | RuleFormatId::EditPath => match_file_rule,
        RuleFormatId::SkillName => |rule, call, _| Ok(match_skill_rule(rule, call)),
        RuleFormatId::SubagentType => |rule, call, _| Ok(match_agent_rule(rule, call)),
        RuleFormatId::Domain => |rule, call, _| Ok(match_web_fetch_rule(rule, call)),
        RuleFormatId::Bare => fails_closed,
    }
}

/// One format's entry of the table, a function the host calls.
#[derive(Debug)]
struct Matcher(RuleFormatId);

crate::registry::methods! {
    Matcher {
        "call" => fallible |this, rule: PermissionRule, call: PreparedCall, ctx: MatchContext|
            specifier_matcher(this.0)(&rule, &call, &ctx);
    }
}

/// The formats of `SPECIFIER_MATCHERS`, in its order.
const SPECIFIER_MATCHERS: [(&str, RuleFormatId); 8] = [
    ("command", RuleFormatId::Command),
    ("powershell-command", RuleFormatId::PowershellCommand),
    ("read-path", RuleFormatId::ReadPath),
    ("edit-path", RuleFormatId::EditPath),
    ("skill-name", RuleFormatId::SkillName),
    ("subagent-type", RuleFormatId::SubagentType),
    ("domain", RuleFormatId::Domain),
    ("bare", RuleFormatId::Bare),
];

crate::registry::exports! {
    "rules/matchers" {
        "SPECIFIER_MATCHERS" => ok || SPECIFIER_MATCHERS
            .map(|(name, format)| (name, Function::new(Matcher(format))))
            .into_iter()
            .collect::<IndexMap<_, _>>();
    }
}
