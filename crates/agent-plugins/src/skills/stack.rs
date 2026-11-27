//! Port of `src/skills/stack.ts`, switched.
//!
//! Several skills in one message (skills.md, "Stack several skills in one message"): at the terminal
//! prompt `/write-tests /fix-issue 123` loads both skills and passes `123` to each. "Claude Code
//! expands the first skill plus up to five more stacked after it. Expansion stops at the first token
//! that isn't an inline user-invocable skill": a skill that runs as a forked subagent, or one whose
//! arguments may start with another command, ends it, and that token and everything after become the
//! argument text for every expanded skill. `readSkillMentions` finds the words; this decides which
//! of them are skills of the session that may be stacked.
//!
//! The catalog is a `StackCatalog`: the host's `SkillLookup` given to `stackSkills` (`skills/lookup`,
//! whose `find` hands back the host's own rows), which crosses as a port: `find` is asked of the host
//! and a row comes back as the value it gave; or the core's own rows (`host/session-mentions`). Its
//! `names` are read once per call.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::invocation::{Invocable, invocation_policy};
use super::mention::{CoreStackable, SkillMention, is_leading, read_skill_mentions_with};
use super::visibility::SkillOverrides;
use crate::js::JsValue;
use crate::ports::BoxFuture;
use crate::registry::{Port, Set};

/// How a stack is read.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackOptions {
    /// True at the terminal prompt (the default); through the Agent SDK or `claude -p` only the
    /// first skill loads.
    #[serde(default)]
    pub interactive: Option<JsValue>,
    /// The `skillOverrides` in force.
    #[serde(default)]
    pub overrides: Option<SkillOverrides>,
    /// The commands whose arguments may start with another command (`/loop`): "expansion ends
    /// there". The kit knows no bundled command, so the host names them.
    #[serde(default)]
    pub takes_command_arguments: Option<Set<String>>,
}

/// The skills a message starts with.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Stack {
    /// The skills to expand, in the order typed: the first, then the ones stacked after it.
    pub skills: Vec<JsValue>,
    /// The text after the last expanded skill: passed as `$ARGUMENTS` to each of them.
    pub arguments: String,
    /// Every mention of the message as written: the direct and stacked ones, then the named ones.
    pub mentions: Vec<SkillMention>,
}

/// Whether a skill may be stacked after another: inline (no `context: fork`), user-invocable, not
/// open-ended.
fn stackable_after(options: &StackOptions, row: &JsValue) -> bool {
    let Some(invocable) = row.get("invocable") else {
        return false;
    };
    let inline = matches!(invocable.get("context"), None | Some(JsValue::Undefined));
    let Ok(skill) = Invocable::deserialize(invocable.clone()) else {
        return false;
    };
    let overrides = options.overrides.clone().unwrap_or_default();
    let open_ended = options
        .takes_command_arguments
        .as_ref()
        .is_some_and(|commands| commands.members.contains(&skill.command));
    inline && invocation_policy(&skill, &overrides).user && !open_ended
}

/// The catalog a message is read against (`SkillLookup`).
pub trait StackCatalog: Send + Sync + 'static {
    /// `find(name)`: the row a name invokes, or `undefined`.
    fn find(&self, name: &str) -> BoxFuture<'_, JsValue>;
    /// `names`: every name that invokes a row.
    fn names(&self) -> BoxFuture<'_, Option<Vec<JsValue>>>;
}

/// The host's `SkillLookup`.
impl StackCatalog for Port {
    fn find(&self, name: &str) -> BoxFuture<'_, JsValue> {
        let name = JsValue::from(name);
        Box::pin(async move { self.method::<JsValue>("find", vec![name]).await })
    }

    fn names(&self) -> BoxFuture<'_, Option<Vec<JsValue>>> {
        Box::pin(async move {
            let names = self.member::<Option<Set<JsValue>>>("names").await;
            names.map(|names| names.members)
        })
    }
}

fn arguments_of(mention: &SkillMention) -> &str {
    match mention {
        SkillMention::Direct { arguments, .. } | SkillMention::Stacked { arguments, .. } => {
            arguments
        }
        SkillMention::Named { .. } => "",
    }
}

pub(crate) fn command_of(mention: &SkillMention) -> &str {
    match mention {
        SkillMention::Direct { command, .. }
        | SkillMention::Stacked { command, .. }
        | SkillMention::Named { command } => command,
    }
}

/// The skills a message starts with and the argument text they share. No skills when the message
/// does not start with a name of the catalog; the host then treats it as text.
pub async fn stack_skills<C: StackCatalog>(
    message: &str,
    catalog: Arc<C>,
    options: StackOptions,
) -> Stack {
    let can_stack = Arc::new(options.clone());
    let asked = Arc::clone(&catalog);
    let stackable: CoreStackable = Arc::new(move |name| {
        let can_stack = Arc::clone(&can_stack);
        let catalog = Arc::clone(&asked);
        Box::pin(async move {
            let row = catalog.find(&name).await;
            !matches!(row, JsValue::Undefined) && stackable_after(&can_stack, &row)
        })
    });
    let names = catalog.names().await;
    let mentions = read_skill_mentions_with(message, options.interactive, names, stackable).await;
    let leading: Vec<&SkillMention> = mentions
        .iter()
        .filter(|mention| is_leading(mention))
        .collect();
    let mut skills = Vec::new();
    for mention in &leading {
        let row = catalog.find(command_of(mention)).await;
        if !matches!(row, JsValue::Undefined) {
            skills.push(row);
        }
    }
    let arguments = match leading.first() {
        Some(first) if skills.len() == leading.len() => arguments_of(first).to_owned(),
        _ => {
            skills.clear();
            String::new()
        }
    };
    Stack {
        skills,
        arguments,
        mentions,
    }
}

crate::registry::exports! {
    "skills/stack" {
        "stackSkills" => async ok |message: String, catalog: Port, options: Option<StackOptions>|
            stack_skills(&message, Arc::new(catalog), options.unwrap_or_default()).await;
    }
}
