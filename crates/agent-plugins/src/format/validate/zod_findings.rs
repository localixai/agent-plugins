//! Ported from `src/format/validate/zod-findings.ts`; only the core uses it now, so it has no wrapper.
//!
//! The findings of a failed zod parse, each at its dotted path. `zodFindings` takes a
//! `safeParse` result, which holds a `ZodError` and so never crosses the boundary: it is a Rust
//! function for the core's ports, not in `exports!`.

use super::finding::{Finding, error_at};
use crate::js::{JsValue, string_of};
use crate::zod::ZodError;

/// The dotted path of a zod issue below `base`, as the oracle prints it (`userConfig.key.title`, `skills.0`).
#[must_use]
pub fn dotted(base: &str, segments: &[JsValue]) -> String {
    std::iter::once(base.to_owned())
        .chain(segments.iter().map(string_of))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(".")
}

/// Every issue of a failed parse as an error at its own path below `base`.
#[must_use]
pub fn zod_findings(parsed: &Result<JsValue, ZodError>, base: &str, suffix: &str) -> Vec<Finding> {
    match parsed {
        Ok(_) => Vec::new(),
        Err(failure) => failure
            .issues
            .iter()
            .map(|found| {
                error_at(
                    &dotted(base, found.path()),
                    &format!("{}{suffix}", found.message()),
                )
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zod;

    #[test]
    fn each_issue_is_an_error_at_its_dotted_path() {
        let schema = zod::object(zod::fields! { "skills" => zod::array(zod::string()) });
        let mut members = crate::js::JsObject::new();
        members.insert(
            "skills".to_owned(),
            JsValue::Array(vec![JsValue::Number(1.0)]),
        );
        let found = zod_findings(&schema.safe_parse(&JsValue::Object(members)), "", "!");
        let paths: Vec<_> = found.iter().map(|finding| finding.path.as_str()).collect();
        assert_eq!(paths, ["skills.0"]);
        assert!(zod_findings(&Ok(JsValue::Null), "x", "").is_empty());
        assert_eq!(dotted("api", &[JsValue::from("oauth")]), "api.oauth");
    }
}
