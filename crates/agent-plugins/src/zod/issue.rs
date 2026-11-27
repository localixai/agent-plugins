//! zod's issues: the raw issue a schema or a check raises, the issue it becomes once its message
//! is given (`util.finalizeIssue`), and the error `parse` throws. An issue keeps its members in
//! zod's own order, which differs per code (`{expected, code}` for a type, `{code, expected}` for
//! a missing member), because the package passes the issues on and a host may print them.

use std::fmt;

use serde::Serialize;

use super::locale;
use super::params::ErrorMap;
use crate::js::{JsObject, JsValue, json_stringify_indented};

/// An issue as it is raised, before its message: zod's `$ZodRawIssue`. Its members are what zod
/// keeps of it (`input`, `inst` and `continue` are kept apart); `input` is the value that raised
/// it, which an error map reads; `error` is the error map of the schema or check that raised it.
#[derive(Clone)]
pub struct RawIssue {
    members: JsObject,
    input: JsValue,
    error: Option<ErrorMap>,
    continues: Option<bool>,
}

/// A raw issue of `code` with no other member yet: what a `.check` or `.superRefine` closure
/// writes as an object literal, `{ code: 'custom', message: '…' }`.
#[must_use]
pub fn issue(code: &str) -> RawIssue {
    RawIssue::new([("code", JsValue::from(code))])
}

impl RawIssue {
    /// An issue with these members, in this order.
    pub(crate) fn new<const N: usize>(members: [(&str, JsValue); N]) -> Self {
        Self {
            members: members
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
            input: JsValue::Undefined,
            error: None,
            continues: None,
        }
    }

    /// The issue with one more member, or `key` set where it already is.
    #[must_use]
    pub fn set(mut self, key: &str, value: impl Into<JsValue>) -> Self {
        self.members.insert(key.to_owned(), value.into());
        self
    }

    /// `{ path: [...] }`.
    #[must_use]
    pub fn path<I: Into<JsValue>>(self, path: impl IntoIterator<Item = I>) -> Self {
        let path: Vec<JsValue> = path.into_iter().map(Into::into).collect();
        self.set("path", path)
    }

    /// `{ message: '…' }`.
    #[must_use]
    pub fn message(self, message: impl Into<String>) -> Self {
        self.set("message", message.into())
    }

    /// `{ input: value }`.
    #[must_use]
    pub fn input(mut self, input: JsValue) -> Self {
        self.input = input;
        self
    }

    /// `{ continue: flag }`: whether the checks after it still run (`true`), or it aborts them.
    #[must_use]
    pub const fn continues(mut self, flag: bool) -> Self {
        self.continues = Some(flag);
        self
    }

    /// The issue raised by a schema or check with this error map (zod's `inst`).
    #[must_use]
    pub(crate) fn raised_by(mut self, error: Option<ErrorMap>) -> Self {
        self.error = error;
        self
    }

    pub(crate) fn with_input_if_unset(mut self, input: &JsValue) -> Self {
        if self.input.is_undefined() {
            self.input = input.clone();
        }
        self
    }

    pub(crate) fn with_error_if_unset(mut self, error: Option<&ErrorMap>) -> Self {
        if self.error.is_none() {
            self.error = error.cloned();
        }
        self
    }

    pub(crate) const fn with_continue_if_unset(mut self, flag: Option<bool>) -> Self {
        if self.continues.is_none() {
            self.continues = flag;
        }
        self
    }

    /// `issue.code`.
    #[must_use]
    pub fn code(&self) -> &str {
        self.members
            .get("code")
            .and_then(JsValue::as_str)
            .unwrap_or_default()
    }

    /// `issue.input`.
    #[must_use]
    pub const fn input_value(&self) -> &JsValue {
        &self.input
    }

    /// A member (`expected`, `values`, `origin`, …).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&JsValue> {
        self.members.get(key)
    }

    /// Whether the checks after it still run: zod's `continue === true`.
    pub(crate) const fn lets_checks_run(&self) -> bool {
        matches!(self.continues, Some(true))
    }

    /// zod's `continue === false`: an explicit abort.
    pub(crate) const fn aborts_explicitly(&self) -> bool {
        matches!(self.continues, Some(false))
    }

    /// The issue one level down, under `key`: `util.prefixIssues`, which puts the key first in a
    /// `path` the issue has, or gives it one at its end.
    #[must_use]
    pub(crate) fn under(mut self, key: JsValue) -> Self {
        match self.members.get_mut("path") {
            Some(JsValue::Array(path)) => path.insert(0, key),
            Some(other) => *other = JsValue::Array(vec![key]),
            None => {
                self.members
                    .insert("path".to_owned(), JsValue::Array(vec![key]));
            }
        }
        self
    }
}

impl fmt::Debug for RawIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RawIssue")
            .field("members", &self.members)
            .field("input", &self.input)
            .field("continues", &self.continues)
            .finish_non_exhaustive()
    }
}

/// An issue as `safeParse` reports it: zod's `$ZodIssue`, `code`, `path` and `message` and the
/// members its code adds (`expected`, `values`, `minimum`, `origin`, `keys`, …), in zod's order.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Issue {
    members: JsObject,
}

impl Issue {
    /// `issue.code`.
    #[must_use]
    pub fn code(&self) -> &str {
        self.members
            .get("code")
            .and_then(JsValue::as_str)
            .unwrap_or_default()
    }

    /// `issue.path`: object keys as strings, array indexes as numbers.
    #[must_use]
    pub fn path(&self) -> &[JsValue] {
        self.members
            .get("path")
            .and_then(JsValue::as_array)
            .map_or(&[], Vec::as_slice)
    }

    /// `issue.message`.
    #[must_use]
    pub fn message(&self) -> &str {
        self.members
            .get("message")
            .and_then(JsValue::as_str)
            .unwrap_or_default()
    }

    /// A member (`expected`, `keys`, …).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&JsValue> {
        self.members.get(key)
    }

    /// The issue as the plain object zod gives.
    #[must_use]
    pub fn to_js_value(&self) -> JsValue {
        JsValue::Object(self.members.clone())
    }
}

/// The issue with its message: `util.finalizeIssue`. A message the issue carries (not empty)
/// wins, then its error map's, then zod's English one; it gets a `path` at its end when it has
/// none, then `message` (in place when it had one).
pub(crate) fn finalize(raw: RawIssue) -> Issue {
    let carried = match raw.members.get("message") {
        Some(JsValue::String(text)) if !text.is_empty() => Some(text.clone()),
        Some(_) | None => None,
    };
    let message = carried
        .or_else(|| raw.error.as_ref().and_then(|map| map(&raw)))
        .unwrap_or_else(|| locale::message(&raw));
    let mut members = raw.members;
    if !members.contains_key("path") {
        members.insert("path".to_owned(), JsValue::Array(Vec::new()));
    }
    members.insert("message".to_owned(), JsValue::String(message));
    Issue { members }
}

/// What `parse` throws and `safeParse` reports on failure: zod's `ZodError`.
#[derive(Clone, Debug, PartialEq)]
pub struct ZodError {
    /// `error.issues`, in the order the schema raised them.
    pub issues: Vec<Issue>,
}

impl ZodError {
    pub(crate) fn of(raw: Vec<RawIssue>) -> Self {
        Self {
            issues: raw.into_iter().map(finalize).collect(),
        }
    }
}

/// `error.message`: the issues as `JSON.stringify(issues, null, 2)` writes them.
impl fmt::Display for ZodError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let issues = JsValue::Array(self.issues.iter().map(Issue::to_js_value).collect());
        formatter.write_str(&json_stringify_indented(&issues, 2).unwrap_or_default())
    }
}

impl std::error::Error for ZodError {}
