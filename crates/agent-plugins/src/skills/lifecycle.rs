//! Port of `src/skills/lifecycle.ts`, switched.
//!
//! What happens to a skill's content once it is invoked (skills.md, "Skill content lifecycle"): the
//! rendered `SKILL.md` "enters the conversation as a single message and stays there across later
//! turns", the file is not re-read, an `allowed-tools` grant clears at the next user message, a
//! re-invocation with identical rendered content adds a short note instead of a second copy, and
//! auto-compaction re-attaches the most recent invocation of each skill within a token budget. Pure:
//! the ledger is a value the host keeps next to its conversation.
//!
//! The ledger crosses as data: a member the TypeScript would carry along in a spread (`...entry`,
//! `...ledger`) and these types do not name is not kept (judgment).

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::format::token_estimate::{CHARS_PER_TOKEN, estimate_tokens};
use crate::js::value::serialize_number;
use crate::js::{math_min, slice_index};
use crate::ports::BoxFuture;
use crate::registry::Port;

/// "keeping the first 5,000 tokens of each".
pub const REATTACH_TOKENS_PER_SKILL: f64 = 5_000.0;
/// "Re-attached skills share a combined budget of 25,000 tokens."
pub const REATTACH_TOKENS_TOTAL: f64 = 25_000.0;
// Judgment (spec/judgments.md): the docs count tokens and give no tokenizer, so the kit counts
// by `estimateTokens` (format/token-estimate.ts); a host with a tokenizer passes its own counter.

/// A token counter: the kit's estimate, or the host's (`TokenCounter`).
pub trait TokenCounter: Send + Sync {
    /// The tokens of `text`.
    fn count<'a>(&'a self, text: &'a str) -> BoxFuture<'a, f64>;
    /// The first `tokens` tokens of `text`.
    fn head<'a>(&'a self, text: &'a str, tokens: f64) -> BoxFuture<'a, String>;
}

/// The host's counter, each member called as a method of it.
impl TokenCounter for Port {
    fn count<'a>(&'a self, text: &'a str) -> BoxFuture<'a, f64> {
        Box::pin(self.method::<f64>("count", vec![text.into()]))
    }

    fn head<'a>(&'a self, text: &'a str, tokens: f64) -> BoxFuture<'a, String> {
        Box::pin(self.method::<String>("head", vec![text.into(), tokens.into()]))
    }
}

/// `CHARACTER_TOKENS`: four characters (code points) to a token.
struct CharacterTokens;

impl TokenCounter for CharacterTokens {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a text's token count is far below 2^53"
    )]
    fn count<'a>(&'a self, text: &'a str) -> BoxFuture<'a, f64> {
        Box::pin(std::future::ready(estimate_tokens(text) as f64))
    }

    #[expect(clippy::cast_precision_loss, reason = "four is exact")]
    fn head<'a>(&'a self, text: &'a str, tokens: f64) -> BoxFuture<'a, String> {
        let characters: Vec<char> = text.chars().collect();
        let end = slice_index(tokens * CHARS_PER_TOKEN as f64, characters.len());
        let head = characters.get(..end).unwrap_or_default().iter().collect();
        Box::pin(std::future::ready(head))
    }
}

/// One invocation that is in the conversation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoadedSkill {
    /// The skill's command.
    pub command: String,
    /// The rendered content, as the model received it.
    pub content: String,
    /// Order of invocation, starting at 1: the most recent has the highest.
    #[serde(serialize_with = "serialize_number")]
    pub order: f64,
}

/// The skills' state of a conversation.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SkillLedger {
    /// The copies of skill content in the conversation, in the order they entered (one skill may have several).
    pub loaded: Vec<LoadedSkill>,
    /// The `allowed-tools` grants in force for the current turn only.
    pub grants: Vec<String>,
    /// The `disallowed-tools` in force for the current turn only.
    pub disallowed: Vec<String>,
}

/// No skill loaded, no grant.
pub const EMPTY_LEDGER: SkillLedger = SkillLedger {
    loaded: Vec::new(),
    grants: Vec::new(),
    disallowed: Vec::new(),
};

/// One invocation of a skill.
#[derive(Clone, Debug, Deserialize)]
pub struct Invocation {
    /// The skill's command.
    pub command: String,
    /// The rendered content: arguments and `` !`command` `` output already in it.
    pub content: String,
    /// Its `allowed-tools`.
    #[serde(default)]
    pub grants: Option<Vec<String>>,
    /// Its `disallowed-tools`.
    #[serde(default)]
    pub disallowed: Option<Vec<String>>,
}

/// What enters the conversation for an invocation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SkillDelivery {
    /// The full content.
    Content {
        /// The content.
        text: String,
    },
    /// The note that it is already loaded.
    AlreadyLoaded {
        /// The note.
        text: String,
    },
}

/// Judgment (spec/judgments.md): the docs say "a short note that the skill is already loaded"
/// without wording.
fn already_loaded_note(command: &str) -> String {
    format!("The skill /{command} is already loaded in this conversation with the same content.")
}

/// The index of the last copy of `command` (`findLast`).
fn latest_of(ledger: &SkillLedger, command: &str) -> Option<usize> {
    ledger
        .loaded
        .iter()
        .rposition(|entry| entry.command == command)
}

/// The ledger after an invocation, and what enters the conversation.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Invoked {
    /// The ledger.
    pub ledger: SkillLedger,
    /// What enters the conversation.
    pub delivery: SkillDelivery,
}

/// Records an invocation. Identical rendered content to the copy already in context gives the note;
/// "When the rendered content differs, because the arguments changed or a dynamic context command
/// produced new output, Claude Code appends the full content again." The invocation re-applies its
/// grants and removals for this turn either way.
#[must_use]
pub fn invoke_skill(ledger: &SkillLedger, invocation: &Invocation) -> Invoked {
    let Invocation {
        command, content, ..
    } = invocation;
    let previous = latest_of(ledger, command);
    let same = previous
        .and_then(|index| ledger.loaded.get(index))
        .is_some_and(|entry| entry.content == *content);
    let order = ledger.loaded.last().map_or(0.0, |entry| entry.order) + 1.0;
    // Judgment (spec/judgments.md): a re-invocation that only adds the note still makes the skill the
    // most recently invoked, so compaction ranks it by that invocation.
    let mut loaded = ledger.loaded.clone();
    match previous.and_then(|index| loaded.get_mut(index)) {
        Some(entry) if same => entry.order = order,
        _ => loaded.push(LoadedSkill {
            command: command.clone(),
            content: content.clone(),
            order,
        }),
    }
    let mut grants = ledger.grants.clone();
    grants.extend(invocation.grants.iter().flatten().cloned());
    let mut disallowed = ledger.disallowed.clone();
    disallowed.extend(invocation.disallowed.iter().flatten().cloned());
    let delivery = if same {
        SkillDelivery::AlreadyLoaded {
            text: already_loaded_note(command),
        }
    } else {
        SkillDelivery::Content {
            text: content.clone(),
        }
    };
    Invoked {
        ledger: SkillLedger {
            loaded,
            grants,
            disallowed,
        },
        delivery,
    }
}

/// The user sends a message: the skills' content stays, but "an `allowed-tools` grant clears when you
/// send your next message", and so does the `disallowed-tools` restriction.
#[must_use]
pub fn start_turn(ledger: SkillLedger) -> SkillLedger {
    SkillLedger {
        grants: Vec::new(),
        disallowed: Vec::new(),
        ..ledger
    }
}

/// How compaction re-attaches.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", bound(deserialize = "C: Deserialize<'de>"))]
pub struct ReattachOptions<C> {
    /// Tokens kept of each skill.
    #[serde(default)]
    pub per_skill_tokens: Option<f64>,
    /// Tokens of all of them together.
    #[serde(default)]
    pub total_tokens: Option<f64>,
    /// The host's counter.
    #[serde(default)]
    pub tokens: Option<C>,
}

impl<C> Default for ReattachOptions<C> {
    fn default() -> Self {
        Self {
            per_skill_tokens: None,
            total_tokens: None,
            tokens: None,
        }
    }
}

/// What auto-compaction carries forward after the summary: the most recent invocation of each skill,
/// the first 5,000 tokens of each, filled into a 25,000-token budget "starting from the most recently
/// invoked skill, so older skills can be dropped entirely". Judgment (spec/judgments.md): a skill
/// that no longer fits is dropped whole, and so is every older one. Returned oldest first, as they
/// were invoked.
pub async fn reattach_after_compaction<C: TokenCounter>(
    ledger: &SkillLedger,
    options: &ReattachOptions<C>,
) -> Vec<LoadedSkill> {
    let per_skill_tokens = options
        .per_skill_tokens
        .unwrap_or(REATTACH_TOKENS_PER_SKILL);
    let total_tokens = options.total_tokens.unwrap_or(REATTACH_TOKENS_TOTAL);
    let counter: &dyn TokenCounter = match &options.tokens {
        Some(tokens) => tokens,
        None => &CharacterTokens,
    };
    let mut commands: Vec<&String> = Vec::new();
    for entry in &ledger.loaded {
        if !commands.contains(&&entry.command) {
            commands.push(&entry.command);
        }
    }
    let mut latest: Vec<&LoadedSkill> = commands
        .into_iter()
        .filter_map(|command| latest_of(ledger, command).and_then(|index| ledger.loaded.get(index)))
        .collect();
    latest.sort_by(|left, right| {
        (right.order - left.order)
            .partial_cmp(&0.0)
            .unwrap_or(Ordering::Equal)
    });
    let mut kept: Vec<LoadedSkill> = Vec::new();
    let mut used = 0.0;
    for entry in latest {
        let content = counter.head(&entry.content, per_skill_tokens).await;
        let size = math_min(counter.count(&content).await, per_skill_tokens);
        if used + size > total_tokens {
            break;
        }
        used += size;
        kept.push(LoadedSkill {
            content,
            ..entry.clone()
        });
    }
    kept.reverse();
    kept
}

crate::registry::exports! {
    "skills/lifecycle" {
        "REATTACH_TOKENS_PER_SKILL" => ok || REATTACH_TOKENS_PER_SKILL;
        "REATTACH_TOKENS_TOTAL" => ok || REATTACH_TOKENS_TOTAL;
        "EMPTY_LEDGER" => ok || EMPTY_LEDGER;
        "invokeSkill" => ok |ledger: SkillLedger, invocation: Invocation| invoke_skill(&ledger, &invocation);
        "startTurn" => ok |ledger: SkillLedger| start_turn(ledger);
        "reattachAfterCompaction" => async ok |ledger: SkillLedger, options: Option<ReattachOptions<Port>>|
            reattach_after_compaction(&ledger, &options.unwrap_or_default()).await;
    }
}
