//! What a closure of a schema is given: the payload of a `.check`, the context of a
//! `.superRefine` and a `.transform` (the value and the issues it adds), and what a `.catch`
//! reads (the input and the issues it caught).

use super::issue::{Issue, RawIssue};
use super::params::ErrorMap;
use crate::js::JsValue;

/// zod's payload as a check, a refinement or a transform sees it: `context.value`,
/// `context.issues.push(...)`, `context.addIssue(...)`.
pub struct Context<'a> {
    value: &'a JsValue,
    issues: Vec<RawIssue>,
    /// What `addIssue` fills in when the issue leaves it out: the error map of the check that
    /// adds it (zod's `inst`) and its `continue`.
    error: Option<ErrorMap>,
    continues: Option<bool>,
}

impl<'a> Context<'a> {
    pub(crate) const fn new(
        value: &'a JsValue,
        error: Option<ErrorMap>,
        continues: Option<bool>,
    ) -> Self {
        Self {
            value,
            issues: Vec::new(),
            error,
            continues,
        }
    }

    /// `context.value`.
    #[must_use]
    pub const fn value(&self) -> &JsValue {
        self.value
    }

    /// `context.issues.push(issue)`: the issue as it is written, nothing filled in.
    pub fn push_issue(&mut self, issue: RawIssue) {
        self.issues.push(issue);
    }

    /// `context.addIssue(issue)`: the issue with what it leaves out filled in, its `input` the
    /// value, its error map the check's, its `continue` the check's (a refinement goes on, a
    /// transform does not).
    pub fn add_issue(&mut self, issue: RawIssue) {
        let filled = issue
            .with_input_if_unset(self.value)
            .with_error_if_unset(self.error.as_ref())
            .with_continue_if_unset(self.continues);
        self.issues.push(filled);
    }

    pub(crate) fn into_issues(self) -> Vec<RawIssue> {
        self.issues
    }
}

/// What a `.catch` function is given: `context.input` (the value its schema answered) and
/// `context.error.issues`.
#[derive(Debug, Clone, PartialEq)]
pub struct CatchContext {
    /// `context.input`.
    pub input: JsValue,
    /// `context.error.issues`, with their messages.
    pub issues: Vec<Issue>,
}
