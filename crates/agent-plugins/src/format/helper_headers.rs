//! Port of `src/format/helper-headers.ts`, switched.
//!
//! What a `headersHelper` prints (mcp.md, "Use dynamic headers for custom authentication": "a JSON
//! object of string key-value pairs to stdout"; plugins/host-marketplace.md, "Write the headersHelper
//! command"): the one reader for an MCP server's helper and a marketplace's. Judgment
//! (spec/judgments.md): the whole of stdout is parsed, and one value that is not a string refuses
//! the output, since the docs name no partial reading.

use crate::js::JsValue;
use indexmap::IndexMap;
use serde::Serialize;

use crate::js::property_order;

/// Why a helper's output was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum HelperHeadersProblem {
    /// The output is not a JSON object.
    #[serde(rename = "not-an-object")]
    NotAnObject,
    /// A value of the object is not a string.
    #[serde(rename = "non-string-value")]
    NonStringValue,
}

/// What a helper's output read as: the headers, or why it was refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum HelperHeadersRead {
    /// The headers, in the order the helper printed them.
    Headers {
        /// Name to value.
        headers: IndexMap<String, String>,
    },
    /// The output was refused.
    Problem {
        /// Why.
        problem: HelperHeadersProblem,
    },
}

/// Reads what a `headersHelper` printed.
#[must_use]
pub fn read_helper_headers(stdout: &str) -> HelperHeadersRead {
    let Ok(JsValue::Object(parsed)) = crate::js::json_parse::json_parse(stdout) else {
        return HelperHeadersRead::Problem {
            problem: HelperHeadersProblem::NotAnObject,
        };
    };
    let entries = parsed.len();
    // `Object.entries` lists integer-like names first (`js::property_order`).
    let strings: IndexMap<String, String> = property_order(&parsed)
        .into_iter()
        .filter_map(|(name, value)| match value {
            JsValue::String(text) => Some((name.clone(), text.clone())),
            JsValue::Undefined
            | JsValue::Null
            | JsValue::Bool(_)
            | JsValue::Number(_)
            | JsValue::Array(_)
            | JsValue::Object(_) => None,
        })
        .collect();
    if strings.len() == entries {
        HelperHeadersRead::Headers { headers: strings }
    } else {
        HelperHeadersRead::Problem {
            problem: HelperHeadersProblem::NonStringValue,
        }
    }
}

/// Whether a header set holds `Authorization`, in any letter case.
#[must_use]
pub fn has_authorization_header(headers: &IndexMap<String, String>) -> bool {
    headers
        .keys()
        .any(|name| name.to_lowercase() == "authorization")
}

crate::registry::exports! {
    "format/helper-headers" {
        "readHelperHeaders" => ok |stdout: String| read_helper_headers(&stdout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // bun -e 'import { readHelperHeaders } from "./src/format/helper-headers.ts";
    //   console.log(JSON.stringify(readHelperHeaders(`{"Authorization":"a","10":"b","2":"c"}`)))'
    // prints {"headers":{"2":"c","10":"b","Authorization":"a"}}
    #[test]
    fn headers_list_integer_like_names_first() {
        let read = read_helper_headers(r#"{"Authorization":"a","10":"b","2":"c"}"#);
        let names: Vec<String> = match read {
            HelperHeadersRead::Headers { headers } => headers.into_keys().collect(),
            HelperHeadersRead::Problem { .. } => Vec::new(),
        };
        assert_eq!(names, ["2", "10", "Authorization"]);
    }
}
