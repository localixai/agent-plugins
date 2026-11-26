//! Differential checks of `js::regexp` against `RegExp` itself, as `scripts/rust/js-oracle.ts`
//! recorded it in the runtime Claude Code runs on.

use agent_plugins::js::regexp::{RegExp, RegExpMatch};
use serde_json::{Value, json};

use crate::support::check_fallible;

/// A match as the oracle records one: `null` for a group that did not take part and for
/// `groups` when the pattern names none.
fn recorded(found: &RegExpMatch) -> Value {
    json!({
        "index": found.index,
        "captures": found.captures,
        "groups": found.groups,
    })
}

#[test]
fn regexp_compiles_and_refuses_as_javascript_core() {
    check_fallible(
        "js/regexp-syntax",
        "RegExp",
        |(source, flags): (String, String)| {
            RegExp::new(&source, &flags).map(|regexp| regexp.flags().to_owned())
        },
    );
}

/// `exec` from a `lastIndex`, with `lastIndex` after it.
fn exec_from(
    source: &str,
    flags: &str,
    text: &str,
    start: usize,
    answer: impl Fn(Option<RegExpMatch>) -> Value,
) -> Result<Value, String> {
    let mut regexp = RegExp::new(source, flags).map_err(|error| error.to_string())?;
    regexp.set_last_index(start);
    let found = regexp.exec(text);
    Ok(json!({ "found": answer(found), "lastIndex": regexp.last_index() }))
}

#[test]
fn regexp_exec_answers_as_javascript_core() {
    check_fallible(
        "js/regexp-exec",
        "RegExp.exec",
        |(source, flags, text, start): (String, String, String, usize)| {
            exec_from(&source, &flags, &text, start, |found| {
                found.as_ref().map_or(Value::Null, recorded)
            })
        },
    );
    check_fallible(
        "js/regexp-exec",
        "RegExp.index",
        |(source, flags, text, start): (String, String, String, usize)| {
            exec_from(&source, &flags, &text, start, |found| {
                found.map_or_else(|| json!(-1), |found| json!(found.index))
            })
        },
    );
}

#[test]
fn regexp_folds_case_as_javascript_core() {
    check_fallible(
        "js/regexp-fold",
        "RegExp.index",
        |(source, flags, text, start): (String, String, String, usize)| {
            exec_from(&source, &flags, &text, start, |found| {
                found.map_or_else(|| json!(-1), |found| json!(found.index))
            })
        },
    );
}

#[test]
fn regexp_replace_answers_as_javascript_core() {
    check_fallible(
        "js/regexp-replace",
        "replace",
        |(source, flags, text, replacement): (String, String, String, String)| {
            let mut regexp = RegExp::new(&source, &flags).map_err(|error| error.to_string())?;
            Ok::<_, String>(regexp.replace(&text, &replacement))
        },
    );
}

#[test]
fn regexp_split_answers_as_javascript_core() {
    check_fallible(
        "js/regexp-split",
        "split",
        |(source, flags, text): (String, String, String)| {
            let regexp = RegExp::new(&source, &flags).map_err(|error| error.to_string())?;
            Ok::<_, String>(regexp.split(&text))
        },
    );
}

#[test]
fn regexp_match_all_answers_as_javascript_core() {
    check_fallible(
        "js/regexp-match-all",
        "matchAll",
        |(source, flags, text): (String, String, String)| {
            let regexp = RegExp::new(&source, &flags).map_err(|error| error.to_string())?;
            let all = regexp.match_all(&text).map_err(str::to_owned)?;
            Ok::<_, String>(all.iter().map(recorded).collect::<Vec<_>>())
        },
    );
}

/// `new RegExp(source, 'u').test(text)` over the texts of `js/reg-exp` (`scripts/rust/js-oracle/reg-exp.ts`),
/// the patterns a schema's `patternProperties` holds: whether it compiles, why not, and what it answers.
#[test]
fn regexp_tests_unicode_patterns_as_javascript_core() {
    check_fallible(
        "js/reg-exp",
        "RegExp",
        |(source, texts): (String, Vec<String>)| {
            let mut regexp = RegExp::new(&source, "u").map_err(|error| error.to_string())?;
            Ok::<_, String>(
                texts
                    .iter()
                    .map(|text| regexp.test(text))
                    .collect::<Vec<bool>>(),
            )
        },
    );
}
