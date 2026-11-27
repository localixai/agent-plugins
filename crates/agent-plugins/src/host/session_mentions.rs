//! Port of `src/host/session-mentions.ts`, switched.
//!
//! What a typed message asks of the skills (skills.md, "Stack several skills in one message",
//! "Where you write the skill's name"): `stackSkills` reads the message against every name of the
//! session's catalog (`skillCatalog`) with the turn's interactivity, and each skill of the stack is
//! expanded by `expandSkill` in order, so hooks, limits and the cap of six stay where they are.
//! Judgment (spec/judgments.md): a leading name that is no skill of the catalog is plain text for the
//! host (a built-in command, a path, a workflow) and is not expanded.
//!
//! The session part is the host's object: its wrapper reads the catalog's rows and the
//! `skillOverrides` in force and passes them, and `expandSkill` is asked of the skill part (a port).

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};
use crate::ports::{BoxFuture, Signal};
use crate::registry::{Port, Set};
use crate::skills::expand_policy::SkillOriginKind;
use crate::skills::lookup::{LookupRow, SkillLookup, skill_lookup};
use crate::skills::mention::{SkillMention, is_leading};
use crate::skills::stack::{StackCatalog, StackOptions, command_of, stack_skills};
use crate::skills::visibility::SkillOverrides;

/// How a message is read: the turn's facts and the host's commands that take another command as
/// their arguments (`MentionOptions`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MentionFacts {
    /// `SessionOptions.interactive`, default `true`.
    #[serde(default)]
    pub interactive: Option<JsValue>,
    /// The `skillOverrides` in force.
    #[serde(default)]
    pub overrides: Option<SkillOverrides>,
    /// The host's commands whose arguments may start with another command (`/loop`).
    #[serde(default)]
    pub takes_command_arguments: Option<Set<String>>,
}

/// Which skills a message names, read against the session's catalog; nothing is expanded.
#[derive(Clone, Debug, Serialize)]
pub struct MessageMentions {
    /// Every skill the message names, in order: the direct one, the stacked ones, then the named ones.
    pub mentions: Vec<SkillMention>,
    /// The listed commands of the direct and stacked skills `expandMessage` expands, in order.
    pub leading: Vec<String>,
    /// The text after the last leading skill: the `$ARGUMENTS` each of them gets.
    pub arguments: String,
    /// The listed commands of the skills the message names after plain text.
    pub permitted: Set<String>,
}

/// One skill of the message, expanded.
#[derive(Clone, Debug, Serialize)]
pub struct MentionExpansion {
    /// The mention.
    pub mention: SkillMention,
    /// What `expandSkill` answered for it.
    pub expansion: JsValue,
}

/// What `expandMessage` answers: the mentions and the expansions of the leading skills.
#[derive(Clone, Debug, Serialize)]
pub struct MessageExpansion {
    /// What `messageMentions` read.
    #[serde(flatten)]
    pub found: MessageMentions,
    /// The direct and stacked skills of the message that exist in the catalog, in order.
    pub expansions: Vec<MentionExpansion>,
}

/// The session's rows (`SessionSkill`) with the lookup over their names.
struct Rows {
    rows: Vec<JsValue>,
    lookup: SkillLookup,
}

fn text_at<'a>(row: &'a JsValue, path: &[&str]) -> &'a str {
    path.iter()
        .try_fold(row, |at, key| at.get(key))
        .and_then(JsValue::as_str)
        .unwrap_or_default()
}

/// `skillLookup(rows)` over the fields it reads of each row.
fn rows_of(rows: Vec<JsValue>) -> Rows {
    let read: Vec<LookupRow> = rows
        .iter()
        .map(|row| LookupRow {
            command: text_at(row, &["invocable", "command"]).to_owned(),
            kind: SkillOriginKind::deserialize(JsValue::from(text_at(
                row,
                &["invocable", "origin", "kind"],
            )))
            .unwrap_or(SkillOriginKind::Plugin),
            names: row
                .get("names")
                .and_then(|names| Vec::<String>::deserialize(names.clone()).ok())
                .unwrap_or_default(),
        })
        .collect();
    let lookup = skill_lookup(&read);
    Rows { rows, lookup }
}

impl Rows {
    fn row(&self, name: &str) -> Option<&JsValue> {
        self.lookup
            .find(name)
            .and_then(|index| self.rows.get(index))
    }
}

impl StackCatalog for Rows {
    fn find(&self, name: &str) -> BoxFuture<'_, JsValue> {
        let row = self.row(name).cloned().unwrap_or_default();
        Box::pin(async move { row })
    }

    fn names(&self) -> BoxFuture<'_, Option<Vec<JsValue>>> {
        let names = self
            .lookup
            .names()
            .map(|name| name.as_str().into())
            .collect();
        Box::pin(async move { Some(names) })
    }
}

/// `read`: the stack of the message against the rows.
async fn read(rows: Arc<Rows>, message: &str, asked: MentionFacts) -> MessageMentions {
    let options = StackOptions {
        interactive: Some(asked.interactive.unwrap_or(JsValue::Bool(true))),
        overrides: asked.overrides,
        takes_command_arguments: asked.takes_command_arguments,
    };
    let stack = stack_skills(message, Arc::clone(&rows), options).await;
    let mut permitted: Vec<String> = Vec::new();
    for mention in stack.mentions.iter().filter(|mention| !is_leading(mention)) {
        let command = command_of(mention);
        let listed = rows
            .row(command)
            .map(|row| text_at(row, &["invocable", "command"]));
        let name = listed.unwrap_or(command).to_owned();
        if !permitted.contains(&name) {
            permitted.push(name);
        }
    }
    MessageMentions {
        leading: stack
            .skills
            .iter()
            .map(|row| text_at(row, &["invocable", "command"]).to_owned())
            .collect(),
        mentions: stack.mentions,
        arguments: stack.arguments,
        permitted: Set { members: permitted },
    }
}

/// What `messageMentions` reads before it expands: the skills `message` names. No port is called.
pub async fn message_mentions(
    rows: Vec<JsValue>,
    message: &str,
    asked: MentionFacts,
) -> MessageMentions {
    read(Arc::new(rows_of(rows)), message, asked).await
}

/// `expandMessage`: each leading skill expanded by the skill part's `expandSkill`, in order.
pub async fn expand_message(
    skills: Port,
    rows: Vec<JsValue>,
    message: &str,
    asked: MentionFacts,
    request: (Option<Signal>, JsValue),
) -> MessageExpansion {
    let (signal, invoker) = request;
    let found = read(Arc::new(rows_of(rows)), message, asked).await;
    let mut options = JsObject::new();
    options.insert("arguments".to_owned(), found.arguments.as_str().into());
    if let Some(signal) = signal.as_ref().and_then(|signal| to_js_value(signal).ok()) {
        options.insert("signal".to_owned(), signal);
    }
    if !matches!(invoker, JsValue::Undefined) {
        options.insert("invoker".to_owned(), invoker);
    }
    let leading: Vec<&SkillMention> = found
        .mentions
        .iter()
        .filter(|mention| is_leading(mention))
        .collect();
    let mut expansions = Vec::new();
    for (index, command) in found.leading.iter().enumerate() {
        let Some(mention) = leading.get(index) else {
            continue;
        };
        let arguments = vec![command.as_str().into(), JsValue::Object(options.clone())];
        let expansion = skills.method::<JsValue>("expandSkill", arguments).await;
        expansions.push(MentionExpansion {
            mention: (*mention).clone(),
            expansion,
        });
    }
    MessageExpansion { found, expansions }
}

crate::registry::exports! {
    "host/session-mentions" {
        "messageMentions" => async ok |rows: Vec<JsValue>, message: String, asked: MentionFacts|
            message_mentions(rows, &message, asked).await;
        "expandMessage" => async ok |skills: Port, rows: Vec<JsValue>, message: String, asked: MentionFacts, signal: Option<Signal>, invoker: JsValue|
            expand_message(skills, rows, &message, asked, (signal, invoker)).await;
    }
}
