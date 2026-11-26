//! Port of `src/format/validate/received.ts`, switched.
//!
//! How the validator names the kind of a JSON value in its messages. Zod's wording ("expected
//! string, received number") uses the bare name; the hooks messages use an article ("received a
//! number").

use crate::js::JsValue;

/// The kind of a value: `null`, `array`, or its `typeof` (`undefined` for a value that is not
/// there).
#[must_use]
pub const fn kind_of(value: &JsValue) -> &'static str {
    match value {
        JsValue::Null => "null",
        JsValue::Array(_) => "array",
        JsValue::Undefined
        | JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::String(_)
        | JsValue::Object(_) => value.type_of(),
    }
}

/// `a string`, `an object`, `null`.
#[must_use]
pub fn with_article(value: &JsValue) -> String {
    let kind = kind_of(value);
    if kind == "null" || kind == "undefined" {
        return kind.to_owned();
    }
    if kind.starts_with(['a', 'e', 'i', 'o', 'u']) {
        format!("an {kind}")
    } else {
        format!("a {kind}")
    }
}

crate::registry::exports! {
    "format/validate/received" {
        "kindOf" => ok |value: JsValue| kind_of(&value).to_owned();
        "withArticle" => ok |value: JsValue| with_article(&value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::JsObject;

    #[test]
    fn a_value_is_named_with_its_article() {
        assert_eq!(with_article(&JsValue::from("x")), "a string");
        assert_eq!(with_article(&JsValue::Object(JsObject::new())), "an object");
        assert_eq!(with_article(&JsValue::Array(Vec::new())), "an array");
        assert_eq!(with_article(&JsValue::Number(f64::NAN)), "a number");
        assert_eq!(with_article(&JsValue::Null), "null");
        assert_eq!(with_article(&JsValue::Undefined), "undefined");
    }
}
