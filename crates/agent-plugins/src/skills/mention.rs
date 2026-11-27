//! Port of `src/skills/mention.ts`, switched.
//!
//! Where a message names a skill (skills.md, "Control who invokes a skill", "Where you write the
//! skill's name"). A name at the start runs the skill directly; after plain text, "as a separate word
//! with no punctuation attached", it only counts as the user's permission for that message. The
//! reader finds the words; whether a skill of that name exists, and may run (`checkInvocation`), is
//! the host's next step.
//!
//! "Stack several skills in one message": at the terminal prompt `/write-tests /fix-issue 123` loads
//! both skills and passes `123` to each; the first plus up to five more are expanded, and expansion
//! stops at the first token that is not an inline user-invocable skill. Through `-p` or the Agent SDK
//! only the first skill loads, with the rest of the line as its arguments.
//!
//! The options' `interactive` and `names` are read once per call, where the TypeScript reads them at
//! each word (judgment: the same answer for any options object that is not a getter).

use std::sync::{Arc, LazyLock};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::js::{JsValue, Pattern, SPACE_CLASS, is_truthy, regex_test, trim, trim_start};
use crate::ports::BoxFuture;
use crate::registry::{Callback, Port, Set};

/// A mention of a skill in a message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "how", rename_all = "lowercase")]
pub enum SkillMention {
    /// The message starts with it: the skill runs.
    Direct {
        /// The skill's command.
        command: String,
        /// The rest of the line.
        arguments: String,
    },
    /// A skill stacked after the direct one: it gets the same trailing `arguments`.
    Stacked {
        /// The skill's command.
        command: String,
        /// The rest of the line.
        arguments: String,
    },
    /// Named after plain text: the user's permission for that message.
    Named {
        /// The skill's command.
        command: String,
    },
}

/// A mention that starts the message (`LeadingMention`): the skill runs.
#[must_use]
pub const fn is_leading(mention: &SkillMention) -> bool {
    !matches!(mention, SkillMention::Named { .. })
}

/// "The first skill plus up to five more stacked after it".
const MAX_STACKED_SKILLS: usize = 6;

/// `MentionOptions` as data: no `stackable` function.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct MentionData {
    /// True at the terminal prompt (the default); false through `-p` or the Agent SDK: only the first skill loads.
    #[serde(default)]
    pub interactive: Option<JsValue>,
    /// The names the session lists, so a directory-qualified name (`/apps/web:deploy`, `/b/cli:verify`)
    /// is one word although it holds `/`: a word that is exactly `/<a listed name>` is a mention.
    #[serde(default)]
    pub names: Option<Set<JsValue>>,
}

/// `MentionOptions`: a record with `stackable` crosses as a port.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum MentionOptions {
    /// Options holding `stackable`: whether a command is an inline, user-invocable skill. A name
    /// that is not (a forked skill such as `/code-review`, `/loop`, an unknown name) ends the stack
    /// and starts the arguments.
    Host(Port),
    /// Options without it: every name qualifies, so the host should pass it.
    Data(MentionData),
}

/// `stackable` as a module of the core gives it (`skills/stack`): whether a command may be stacked.
pub type CoreStackable = Arc<dyn Fn(String) -> BoxFuture<'static, bool> + Send + Sync>;

/// `stackable`: the host's function, a method of its options, or the core's own.
enum Stackable {
    Host(Port),
    Core(CoreStackable),
}

/// The options as one call reads them.
struct Read {
    interactive: Option<JsValue>,
    names: Option<Vec<JsValue>>,
    stackable: Option<Stackable>,
}

impl Read {
    async fn of(options: Option<MentionOptions>) -> Self {
        match options {
            None => Self {
                interactive: None,
                names: None,
                stackable: None,
            },
            Some(MentionOptions::Data(data)) => Self {
                interactive: data.interactive,
                names: data.names.map(|names| names.members),
                stackable: None,
            },
            Some(MentionOptions::Host(port)) => {
                let interactive = port.member::<Option<JsValue>>("interactive");
                let names = port.member::<Option<Set<JsValue>>>("names");
                let stackable = port.member::<Option<Callback>>("stackable");
                Self {
                    interactive: interactive.await,
                    names: names.await.map(|names| names.members),
                    stackable: stackable.await.map(|_| Stackable::Host(port)),
                }
            }
        }
    }
}

/// A word that is a skill name and nothing else: letters, digits, `_`, `-` and the plugin `:`. Any
/// other character in the word is "punctuation attached" (`/deploy,`, `/usr/bin`) and the word is
/// not a mention. Judgment: the docs give the rule, not the alphabet of a name.
static NAME_WORD: Pattern = LazyLock::new(|| Regex::new(r"^[\p{L}\p{N}_:-]+$").ok());
/// `/\S+/gu`.
static WORD: Pattern = LazyLock::new(|| Regex::new(&format!("[^{SPACE_CLASS}]+")).ok());

/// `[...text.matchAll(WORD)]`: each word with where it ends.
fn words_of(text: &str) -> Vec<(&str, usize)> {
    WORD.as_ref().map_or_else(Vec::new, |word| {
        word.find_iter(text)
            .map(|found| (found.as_str(), found.end()))
            .collect()
    })
}

fn name_of(word: &str, options: &Read) -> Option<String> {
    let name = word.strip_prefix('/')?;
    let listed = options
        .names
        .as_ref()
        .is_some_and(|names| names.iter().any(|listed| listed.as_str() == Some(name)));
    (regex_test(&NAME_WORD, name) || listed).then(|| name.to_owned())
}

/// `options.stackable?.(command) ?? true`, read for its truthiness.
async fn is_stackable(options: &Read, command: &str) -> bool {
    let port = match &options.stackable {
        None => return true,
        Some(Stackable::Core(stackable)) => return stackable(command.to_owned()).await,
        Some(Stackable::Host(port)) => *port,
    };
    let answer = port
        .method::<JsValue>("stackable", vec![command.into()])
        .await;
    matches!(answer, JsValue::Undefined | JsValue::Null) || is_truthy(&answer)
}

/// The names stacked after the first, up to the limit of six skills in all, ending at the first that cannot be.
async fn stacked_names(first: &str, following: &[&str], options: &Read) -> Vec<String> {
    if options.interactive == Some(JsValue::Bool(false)) || !is_stackable(options, first).await {
        return Vec::new();
    }
    let mut stacked = Vec::new();
    for word in following.iter().take(MAX_STACKED_SKILLS - 1) {
        let Some(name) = name_of(word, options) else {
            break;
        };
        if !is_stackable(options, &name).await {
            break;
        }
        stacked.push(name);
    }
    stacked
}

/// The mentions of the skills at the start of the message: the direct one and the stacked ones.
async fn read_leading(message: &str, options: &Read) -> Vec<SkillMention> {
    let text = trim_start(message);
    let words = words_of(text);
    let Some(direct) = name_of(words.first().map_or("", |(word, _)| word), options) else {
        return Vec::new();
    };
    let following: Vec<&str> = words.iter().skip(1).map(|(word, _)| *word).collect();
    let stacked = stacked_names(&direct, &following, options).await;
    let rest = words
        .get(stacked.len())
        .and_then(|(_, end)| text.get(*end..))
        .map_or_else(String::new, |rest| trim(rest).to_owned());
    let mut mentions = vec![SkillMention::Direct {
        command: direct,
        arguments: rest.clone(),
    }];
    mentions.extend(stacked.into_iter().map(|command| SkillMention::Stacked {
        command,
        arguments: rest.clone(),
    }));
    mentions
}

/// The command of a mention.
fn command_of(mention: &SkillMention) -> &str {
    match mention {
        SkillMention::Direct { command, .. }
        | SkillMention::Stacked { command, .. }
        | SkillMention::Named { command } => command,
    }
}

/// The skills a message names: the leading ones, then each other name once.
pub async fn read_skill_mentions(
    message: &str,
    options: Option<MentionOptions>,
) -> Vec<SkillMention> {
    mentions_of(message, Read::of(options).await).await
}

/// `readSkillMentions` as a module of the core calls it, with its own `stackable` (`skills/stack`).
pub async fn read_skill_mentions_with(
    message: &str,
    interactive: Option<JsValue>,
    names: Option<Vec<JsValue>>,
    stackable: CoreStackable,
) -> Vec<SkillMention> {
    let options = Read {
        interactive,
        names,
        stackable: Some(Stackable::Core(stackable)),
    };
    mentions_of(message, options).await
}

async fn mentions_of(message: &str, options: Read) -> Vec<SkillMention> {
    let mut mentions = read_leading(message, &options).await;
    let mut seen: Vec<String> = mentions
        .iter()
        .map(|mention| command_of(mention).to_owned())
        .collect();
    for (word, _) in words_of(message) {
        let Some(name) = name_of(word, &options) else {
            continue;
        };
        if seen.contains(&name) {
            continue;
        }
        seen.push(name.clone());
        mentions.push(SkillMention::Named { command: name });
    }
    mentions
}

crate::registry::exports! {
    "skills/mention" {
        "isLeading" => ok |mention: SkillMention| is_leading(&mention);
        "readSkillMentions" => async ok |message: String, options: Option<MentionOptions>|
            read_skill_mentions(&message, options).await;
    }
}
