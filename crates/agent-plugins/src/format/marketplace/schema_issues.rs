//! Port of `src/format/marketplace/schema-issues.ts`.
//!
//! Reports of a zod parse as plugin issues: the validator's own wording ("Invalid input: expected
//! string, received number") at the path of the value, written the way the docs write an entry's
//! paths (`plugins[1].tags[0]`).
//!
//! Its one export takes a `ZodError`, a value of zod's that never crosses the boundary, so it
//! declares no `exports!`: the core's readers call it, and the TypeScript keeps its own until its
//! importers are switched.

use crate::format::issues::{PluginIssue, error};
use crate::js::{JsValue, number_to_string};
use crate::zod::ZodError;

/// `base` followed by zod's path segments: an index is `[n]`, a key is `.key`.
fn path_of(base: &str, segments: &[JsValue]) -> String {
    segments
        .iter()
        .fold(base.to_owned(), |path, segment| match segment {
            JsValue::Number(index) => format!("{path}[{}]", number_to_string(*index)),
            JsValue::String(key) => format!("{path}.{key}"),
            _ => path,
        })
}

/// One error per problem zod found, each at its own path below `base`.
#[must_use]
pub fn zod_issues(failure: &ZodError, base: &str) -> Vec<PluginIssue> {
    failure
        .issues
        .iter()
        .map(|found| error(&path_of(base, found.path()), found.message()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::zod_issues;
    use crate::js::JsValue;
    use crate::zod;

    #[test]
    fn writes_indexes_in_brackets_and_keys_after_a_dot() {
        let schema = zod::object(zod::fields! { "tags" => zod::array(zod::string()) });
        let value = JsValue::Object(
            [(
                "tags".to_owned(),
                JsValue::Array(vec![JsValue::String("a".to_owned()), JsValue::Number(1.0)]),
            )]
            .into_iter()
            .collect(),
        );
        let failure = schema.safe_parse(&value).unwrap_err();
        let issues = zod_issues(&failure, "plugins[1]");
        let paths: Vec<&str> = issues.iter().map(|found| found.path.as_str()).collect();
        assert_eq!(paths, ["plugins[1].tags[1]"]);
    }
}
