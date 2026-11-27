//! Port of `src/format/agent-scalars.ts`, switched.
//!
//! The coercers of an agent's scalar frontmatter fields (sub-agents.md, "Supported frontmatter
//! fields"), each over a shared reader of `frontmatter_values`.

use serde::Serialize;

use super::frontmatter_values::{
    FieldRead, IntegerBounds, ListOptions, Separators, read_enum, read_flag, read_integer,
    read_list, read_table, read_text,
};
use super::issues::{PluginIssue, warning};
use crate::js::{
    JsValue, is_safe_integer, is_whitespace, json_stringify, number, number_to_string, trim,
};
use crate::protocol::effort::EFFORT_LEVELS;

/// What one agent field read gives: its value, or `None` with the issues that say why.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AgentFieldRead<T> {
    /// The value, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<T>,
    /// What was wrong, as warnings.
    pub issues: Vec<PluginIssue>,
}

/// A value of the wrong shape is dropped with a warning and the agent still loads: sub-agents.md
/// says only that Claude Code "ignores a field it doesn't recognize without reporting an error",
/// and, for `autoCompactWindow`, that a bad value loads "as if the field were absent". The same is
/// the kit's judgment for every field the docs give no invalid-value rule (spec/judgments.md).
fn ignored(path: &str, key: &str, reason: &str) -> Vec<PluginIssue> {
    vec![warning(
        path,
        &format!("Invalid \"{key}\": it {reason}; the field is ignored"),
    )]
}

/// Adapts a shared coercion (`frontmatter_values`) to the agent shape.
fn adapt<T>(path: &str, read: FieldRead<T>) -> AgentFieldRead<T> {
    let issues = match &read.problem {
        None => Vec::new(),
        Some(problem) => ignored(path, &problem.key, &problem.reason),
    };
    AgentFieldRead {
        value: read.value,
        issues,
    }
}

/// A text field.
pub fn text_field(path: &str, key: &str, raw: &JsValue) -> AgentFieldRead<String> {
    adapt(path, read_text(key, raw))
}

/// `model`: any non-empty text (spec §5.4.3, J5.5). An empty or blank value is dropped like any
/// bad value, with the warning the client logs, and the default applies.
pub fn model_field(path: &str, key: &str, raw: &JsValue) -> AgentFieldRead<String> {
    let blank = raw.as_str().is_some_and(|text| trim(text).is_empty());
    if blank {
        AgentFieldRead {
            value: None,
            issues: ignored(path, key, "is empty"),
        }
    } else {
        text_field(path, key, raw)
    }
}

/// `background`, `omitClaudeMd`: sub-agents.md says "Set to `true`", so only a YAML boolean or the
/// text `true` or `false` is a flag here; `yes`, `on`, `1` and `0` belong to skills and commands
/// (judgment, spec/judgments.md).
pub fn bool_field(path: &str, key: &str, raw: &JsValue) -> AgentFieldRead<bool> {
    let text = raw.as_str().map(|text| trim(text).to_lowercase());
    let spelled = text
        .as_deref()
        .is_none_or(|text| matches!(text, "" | "true" | "false"));
    if raw.is_number() || !spelled {
        return AgentFieldRead {
            value: None,
            issues: ignored(path, key, "is not a flag: use true or false"),
        };
    }
    adapt(path, read_flag(key, raw))
}

/// `tools`, `disallowedTools`, `skills`: "a comma-separated string ... or a YAML list".
pub fn token_list(path: &str, key: &str, raw: &JsValue) -> AgentFieldRead<Vec<String>> {
    adapt(
        path,
        read_list(
            key,
            raw,
            ListOptions {
                separators: Separators::Comma,
                braces: None,
            },
        ),
    )
}

/// A whole number of at least 1.
pub fn positive_int(path: &str, key: &str, raw: &JsValue) -> AgentFieldRead<f64> {
    adapt(
        path,
        read_integer(
            key,
            raw,
            IntegerBounds {
                min: Some(1.0),
                max: None,
            },
        ),
    )
}

/// One of `allowed`, exactly.
pub fn enum_field<'a>(
    allowed: &'a [&'a str],
) -> impl Fn(&str, &str, &JsValue) -> AgentFieldRead<String> + 'a {
    move |path, key, raw| adapt(path, read_enum(key, raw, allowed))
}

/// sub-agents.md, "Choose an effort level": `low`, `medium`, `high`, `xhigh` or `max`.
pub fn effort_field(path: &str, key: &str, raw: &JsValue) -> AgentFieldRead<String> {
    enum_field(EFFORT_LEVELS)(path, key, raw)
}

/// The range `autoCompactWindow` may hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct AutoCompactRange {
    /// The least.
    pub min: u32,
    /// The most.
    pub max: u32,
}

/// sub-agents.md, "Set the auto-compact window for one subagent": a token count in this range.
pub const AUTO_COMPACT_RANGE: AutoCompactRange = AutoCompactRange {
    min: 100_000,
    max: 1_000_000,
};

/// `^\s*[+-]?\d+(?!\.\d)`: the leading integer. The engine backs off a digit when a fraction
/// follows (`"200000.5"` reads as `20000`), as the lookahead does.
fn leading_integer(text: &str) -> Option<&str> {
    let start = text.len() - text.trim_start_matches(is_whitespace).len();
    let rest = text.get(start..)?;
    let sign = usize::from(rest.starts_with(['+', '-']));
    let digits = rest
        .get(sign..)?
        .bytes()
        .take_while(u8::is_ascii_digit)
        .count();
    let fraction_follows = |count: usize| {
        let after = rest.get(sign + count..).unwrap_or("");
        after.starts_with('.') && after.as_bytes().get(1).is_some_and(u8::is_ascii_digit)
    };
    let taken = if fraction_follows(digits) {
        digits.checked_sub(1).filter(|count| *count > 0)?
    } else {
        digits
    };
    if taken == 0 {
        return None;
    }
    rest.get(..sign + taken)
}

/// What Claude Code reads from the field: a YAML number as it is, and a string as its leading
/// integer, because "in frontmatter Claude Code reads `200k` as 200". YAML 1.2 reads `200k` as a
/// string, so the leading integer is how the kit reaches the documented 200. A value with no
/// leading integer, or whose integer is followed by a fraction (`"200000.5"`), reads as nothing,
/// the same as the unquoted number (judgment, spec/judgments.md).
fn window_tokens(raw: &JsValue) -> Option<f64> {
    if let Some(tokens) = raw.as_f64() {
        return Some(tokens);
    }
    leading_integer(raw.as_str()?).map(number)
}

/// `autoCompactWindow`: from 100000 to 1000000. A value out of that range (or not a number)
/// leaves the subagent loading "as if the field were absent", reported with the words the docs
/// tell the author to search the debug log for: `invalid autoCompactWindow`.
pub fn auto_compact_window(path: &str, key: &str, raw: &JsValue) -> AgentFieldRead<f64> {
    if raw.is_nullish() {
        return AgentFieldRead {
            value: None,
            issues: Vec::new(),
        };
    }
    let tokens = window_tokens(raw);
    let AutoCompactRange { min, max } = AUTO_COMPACT_RANGE;
    if let Some(tokens) = tokens
        && is_safe_integer(tokens)
        && tokens >= f64::from(min)
        && tokens <= f64::from(max)
    {
        return AgentFieldRead {
            value: Some(tokens),
            issues: Vec::new(),
        };
    }
    let read = tokens.map_or_else(
        || "not a number".to_owned(),
        |tokens| format!("read as {}", number_to_string(tokens)),
    );
    let shown = json_stringify(raw).unwrap_or_else(|| "undefined".to_owned());
    let message =
        format!("invalid {key} {shown}: {read}, outside {min} to {max}; the field is ignored");
    AgentFieldRead {
        value: None,
        issues: vec![warning(path, &message)],
    }
}

/// `experimental.cacheTtl`: "`5m` or `1h`".
const CACHE_TTLS: [&str; 2] = ["5m", "1h"];

/// What the kit keeps of the `experimental` map: only `cacheTtl`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Experimental {
    /// `5m` or `1h`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_ttl: Option<String>,
}

/// The `experimental` map: "Claude Code ignores any other value", so another key is dropped
/// silently, and a `cacheTtl` that is not `5m` or `1h` is dropped with a warning (judgment).
pub fn experimental_map(path: &str, key: &str, raw: &JsValue) -> AgentFieldRead<Experimental> {
    let table = read_table(key, raw);
    let Some(members) = table.value else {
        return adapt(
            path,
            FieldRead {
                value: None,
                problem: table.problem,
            },
        );
    };
    let ttl = read_enum(
        &format!("{key}.cacheTtl"),
        members.get("cacheTtl").unwrap_or(&JsValue::Undefined),
        &CACHE_TTLS,
    );
    let AgentFieldRead { value, issues } = adapt(path, ttl);
    AgentFieldRead {
        value: Some(Experimental { cache_ttl: value }),
        issues,
    }
}

crate::registry::exports! {
    "format/agent-scalars" {
        "textField" => ok |path: String, key: String, raw: JsValue| text_field(&path, &key, &raw);
        "modelField" => ok |path: String, key: String, raw: JsValue| model_field(&path, &key, &raw);
        "boolField" => ok |path: String, key: String, raw: JsValue| bool_field(&path, &key, &raw);
        "tokenList" => ok |path: String, key: String, raw: JsValue| token_list(&path, &key, &raw);
        "positiveInt" => ok |path: String, key: String, raw: JsValue| positive_int(&path, &key, &raw);
        "enumField" => ok |allowed: Vec<String>, path: String, key: String, raw: JsValue| {
            let allowed: Vec<&str> = allowed.iter().map(String::as_str).collect();
            enum_field(&allowed)(&path, &key, &raw)
        };
        "effortField" => ok |path: String, key: String, raw: JsValue| effort_field(&path, &key, &raw);
        "AUTO_COMPACT_RANGE" => ok || AUTO_COMPACT_RANGE;
        "autoCompactWindow" => ok |path: String, key: String, raw: JsValue| auto_compact_window(&path, &key, &raw);
        "experimentalMap" => ok |path: String, key: String, raw: JsValue| experimental_map(&path, &key, &raw);
    }
}
