//! Ported from `src/hooks/output/merge-rules.ts`; only the core uses it now, so it has no wrapper.
//!
//! What the per-event merge rules share: one hook's output as the rules read it, and what a rule
//! returns. A rule gets every hook's read, in registry order (the order hooks are registered in, so
//! the result does not depend on which hook finishes first). `Merger` is a type and stays in the
//! package's TypeScript; `Read` is declared here for the core's merges, which build it.

use serde::{Deserialize, Serialize};

use crate::format::issues::PluginIssue;
use crate::hooks::contracts::HookSource;
use crate::hooks::verdict::types::MergedBlock;
use crate::js::{JsObject, JsValue};

/// The part of a registered handler the merges name.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ReadHandler {
    /// The handler's id.
    pub id: String,
    /// Where it was declared.
    pub source: HookSource,
}

/// What the verdict stage classified a hook's plain stdout as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlainTextAs {
    /// Context for Claude.
    Context,
    /// `PreCompact` instructions.
    CompactionInstructions,
}

/// A hook's plain stdout, as the verdict stage classified it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlainText {
    /// The text.
    pub text: String,
    /// What it is.
    #[serde(rename = "as")]
    pub kind: PlainTextAs,
}

/// One hook's output as the rules read it.
#[derive(Debug, Clone, PartialEq)]
pub struct Read {
    /// The handler.
    pub handler: ReadHandler,
    /// The validated JSON output, `None` for a hook with none.
    pub output: Option<JsObject>,
    /// `hookSpecificOutput` when the event honours it (the event's cap), else `None`.
    pub specific: Option<JsValue>,
    /// The hook's plain stdout, as the verdict stage classified it.
    pub plain_text: Option<PlainText>,
    /// A Monitor call reached the hook through a `Bash` matcher: its `allow` and `updatedInput`
    /// do not apply.
    pub effects_ignored: bool,
    /// The hook itself held the action back: by exit 2, a failure that blocks, or `decision:
    /// "block"`.
    pub blocks: bool,
    /// Raw stdout of a `WorktreeCreate` command hook: its last non-empty line is the path.
    pub stdout: Option<String>,
}

impl Read {
    /// The output's member `key`, `None` when there is no output or no such member.
    #[must_use]
    pub fn output_field(&self, key: &str) -> Option<&JsValue> {
        self.output
            .as_ref()?
            .get(key)
            .filter(|value| !value.is_undefined())
    }

    /// The output's member `key` when it is a string.
    #[must_use]
    pub fn output_text(&self, key: &str) -> Option<String> {
        self.output_field(key)
            .and_then(JsValue::as_str)
            .map(str::to_owned)
    }

    /// `read.output?.decision === 'block'`.
    #[must_use]
    pub fn decides_block(&self) -> bool {
        self.output_text("decision").as_deref() == Some("block")
    }

    /// `read.output?.continue === false`.
    #[must_use]
    pub fn stops(&self) -> bool {
        self.output_field("continue").and_then(JsValue::as_bool) == Some(false)
    }
}

/// What a rule returns, with the block that holds the action back.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MergedRule {
    /// The merged effects.
    pub effects: JsObject,
    /// The issues of the merge.
    pub issues: Vec<PluginIssue>,
    /// A decision that holds the action back (a `deny`, a declined elicitation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block: Option<MergedBlock>,
}

impl From<Merged> for MergedRule {
    fn from(merged: Merged) -> Self {
        Self {
            effects: merged.effects,
            issues: merged.issues,
            block: None,
        }
    }
}

/// What a rule returns, here without a block: `NOTHING` has none.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Merged {
    /// The merged effects.
    pub effects: JsObject,
    /// The issues of the merge.
    pub issues: Vec<PluginIssue>,
}

/// `NOTHING`: the merge of no output at all.
#[must_use]
pub fn nothing() -> Merged {
    Merged {
        effects: JsObject::new(),
        issues: Vec::new(),
    }
}

/// The last defined value: "choose last-in-registry-order wins" (spec/judgments.md). `value ??
/// last`: `null` and `undefined` are not defined.
#[must_use]
pub fn last_of(values: &[JsValue]) -> Option<JsValue> {
    values.iter().fold(None, |last, value| match value {
        JsValue::Undefined | JsValue::Null => last,
        JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::String(_)
        | JsValue::Array(_)
        | JsValue::Object(_) => Some(value.clone()),
    })
}
