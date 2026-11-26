//! Port of `src/rules/tools/semantics/env-number.ts`, switched.
//!
//! The numbers a tool reads from the session's environment.

use crate::js::{is_safe_integer, number, trim};
use crate::ports::environment::Environment;
use crate::registry::Port;

/// `/^\d+$/`: plain ASCII digits.
fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// The value of a variable that is plain digits, zero included; any other spelling is unset.
pub async fn whole_number(env: &dyn Environment, name: &str) -> Option<f64> {
    let text = env.get(name).await?;
    let text = trim(&text);
    if !is_digits(text) {
        return None;
    }
    let value = number(text);
    is_safe_integer(value).then_some(value)
}

/// The value of a variable that is a positive integer; any other text counts as unset
/// (spec/judgments.md: the docs name no behaviour for a variable that is not a number).
pub async fn positive_integer(env: &dyn Environment, name: &str) -> Option<f64> {
    whole_number(env, name).await.filter(|value| *value > 0.0)
}

crate::registry::exports! {
    "rules/tools/semantics/env-number" {
        "positiveInteger" => async ok |env: Port, name: String| positive_integer(&env, &name).await;
    }
}
