//! Port of `src/rules/tools/inputs/ask-user-question.ts`, switched.
//!
//! AskUserQuestion's input (hooks.md, "AskUserQuestion"): "one to four" questions, "each with a
//! `question` string, short `header`, `options` array, and optional `multiSelect` flag". The table
//! says only "options array"; two to four options per question is the SDK's input type
//! (spec/judgments.md). The `header` limit of 12 characters the SDK's comment gives is not enforced:
//! the docs say "short", and a long chip label still asks the question.
//!
//! The input is held as the object it is (`T extends AskedQuestions` keeps every other member), its
//! members in the order the host wrote them.

use std::sync::LazyLock;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::field_schemas::{FLAG, TEXT, list, record};
use crate::format::issues::{PluginIssue, error};
use crate::js::{JsObject, JsValue, in_property_order};
use crate::zod::{self, Schema, fields};

static OPTION: LazyLock<Schema> = LazyLock::new(|| {
    zod::loose_object(fields! {
        "label" => TEXT.clone(),
        "description" => TEXT.clone(),
        "preview" => TEXT.optional(),
    })
});

/// A count between `least` and `most`. A refinement and not `.min().max()`: those length checks also
/// run on a string, which would add a second issue to a value that is not an array.
#[derive(Clone)]
struct Between {
    least: usize,
    most: usize,
    params: String,
}

impl Between {
    fn check(&self, items: &JsValue) -> bool {
        let length = items.as_array().map_or(0, Vec::len);
        length >= self.least && length <= self.most
    }

    /// `list(item).refine(between.check, between.params)`.
    fn refine(&self, schema: &Schema) -> Schema {
        let count = self.clone();
        schema.refine(move |items| count.check(items), self.params.clone())
    }
}

fn between(least: usize, most: usize, what: &str) -> Between {
    Between {
        least,
        most,
        params: format!("expected {least} to {most} {what}"),
    }
}

static QUESTION: LazyLock<Schema> = LazyLock::new(|| {
    let options = between(2, 4, "options");
    zod::loose_object(fields! {
        "question" => TEXT.clone(),
        "header" => TEXT.clone(),
        "options" => options.refine(&list(OPTION.clone())),
        "multiSelect" => FLAG.optional(),
    })
});

/// `questionsSchema`: one to four questions.
pub static QUESTIONS_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    let questions = between(1, 4, "questions");
    questions.refine(&list(QUESTION.clone()))
});

/// `answersSchema`: "Maps question text to the selected option label".
pub static ANSWERS_SCHEMA: LazyLock<Schema> = LazyLock::new(|| record(TEXT.clone()));

/// What answering gives: the whole input with `answers`, ready to be a hook's `updatedInput`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnsweredQuestions {
    /// The whole input with `answers`.
    pub input: JsObject,
    /// What was refused.
    pub issues: Vec<PluginIssue>,
}

fn rejected(path: &str, message: &str) -> PluginIssue {
    error(&format!("tool_input.answers.{path}"), message)
}

/// Answers a call programmatically (hooks.md, "Tools that require user interaction"): "echo back the
/// original `questions` array and add an `answers` object mapping each question's text to the chosen
/// answer", the labels of a multi-select question joined with commas. `selections` maps a question's
/// text to what the user chose. Judgments (spec/judgments.md): a selection for a question that was
/// not asked, or two or more for a single-select question, is an issue and adds nothing; no
/// selection leaves the question unanswered; a text that is no option label stays, because the user
/// can write their own answer; answers already in the input stay unless a selection replaces them.
#[must_use]
pub fn answer_questions(
    input: &JsObject,
    selections: &IndexMap<String, Vec<String>>,
) -> AnsweredQuestions {
    let mut asked: IndexMap<&str, bool> = IndexMap::new();
    for entry in input
        .get("questions")
        .and_then(JsValue::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(question) = entry.get("question").and_then(JsValue::as_str) {
            let multiple = entry.get("multiSelect").and_then(JsValue::as_bool) == Some(true);
            asked.insert(question, multiple);
        }
    }
    let mut answers: JsObject = input
        .get("answers")
        .and_then(JsValue::as_object)
        .cloned()
        .unwrap_or_default();
    let mut issues: Vec<PluginIssue> = Vec::new();
    for (question_text, chosen) in selections {
        match asked.get(question_text.as_str()) {
            None => issues.push(rejected(question_text, "is not one of the questions asked")),
            Some(false) if chosen.len() > 1 => issues.push(rejected(
                question_text,
                "is a single-select question and takes one answer",
            )),
            Some(_) if !chosen.is_empty() => {
                answers.insert(question_text.clone(), JsValue::from(chosen.join(",")));
            }
            Some(_) => {}
        }
    }
    let mut answered = input.clone();
    answered.insert(
        "answers".to_owned(),
        JsValue::from(in_property_order(answers)),
    );
    AnsweredQuestions {
        input: in_property_order(answered),
        issues,
    }
}

crate::registry::exports! {
    "rules/tools/inputs/ask-user-question" {
        "answerQuestions" => ok |input: JsObject, selections: IndexMap<String, Vec<String>>|
            answer_questions(&input, &selections);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages(schema: &Schema, value: &JsValue) -> Vec<(String, String)> {
        schema
            .safe_parse(value)
            .err()
            .map_or_else(Vec::new, |failure| {
                failure
                    .issues
                    .iter()
                    .map(|found| {
                        let path: Vec<String> =
                            found.path().iter().map(crate::js::string_of).collect();
                        (path.join("."), found.message().to_owned())
                    })
                    .collect()
            })
    }

    #[test]
    fn a_count_out_of_range_is_one_issue_and_a_string_is_only_a_wrong_type() {
        let none = JsValue::from(Vec::new());
        assert_eq!(
            messages(&QUESTIONS_SCHEMA, &none),
            [(String::new(), "expected 1 to 4 questions".to_owned())]
        );
        assert_eq!(
            messages(&QUESTIONS_SCHEMA, &JsValue::from("four")),
            [(String::new(), "expected an array".to_owned())]
        );
        let mut answers = JsObject::new();
        answers.insert("Which?".to_owned(), JsValue::from(1.0));
        assert_eq!(
            messages(&ANSWERS_SCHEMA, &JsValue::from(answers)),
            [("Which?".to_owned(), "expected a string".to_owned())]
        );
    }
}
