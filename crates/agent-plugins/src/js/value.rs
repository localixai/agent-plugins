//! A JavaScript value: what the npm package's TypeScript types `unknown`, and what `JSON.parse`
//! or the YAML reader of a frontmatter gives. JSON's values do not hold all of it: a number is
//! one double, so `NaN`, `Infinity`, `-Infinity` and `-0` are numbers too (YAML's `.nan`, `.inf`,
//! `-.inf` and `-0`, `JSON.parse("-0")`), and `undefined` may be an array item or an object
//! member. An object keeps its members in the order they were written; the order JavaScript lists
//! them in is [`property_order`](crate::js::property_order)'s to give.
//!
//! Its serde spelling is the wire's (`src/wasm/wire.ts`, `scripts/rust/recorder/encode.ts`):
//! JSON, with what JSON cannot spell tagged, `{"$undefined": true}` and `{"$number": "NaN" |
//! "Infinity" | "-Infinity" | "-0"}`; a finite number is a JSON number, an integer one written
//! without a fraction. [`to_js_value`] reads anything serializable into a value, a value is itself
//! a serde deserializer for a port's typed parameters, and [`JsValue::to_json_text`] writes the
//! wire's text with each number as JavaScript writes it.

mod de;
mod ser;
mod text;

pub use ser::{serialize_number, to_js_value};
pub(crate) use text::write_json_string;

use indexmap::IndexMap;

/// The members of a JavaScript object, in the order they were written.
pub type JsObject = IndexMap<String, JsValue>;

/// One JavaScript value (the module comment).
#[derive(Debug, Clone, Default, PartialEq)]
pub enum JsValue {
    /// `undefined`: an argument left out, a member or item set to it.
    #[default]
    Undefined,
    /// `null`.
    Null,
    /// A boolean.
    Bool(bool),
    /// A number: any double, `NaN`, the infinities and `-0` included.
    Number(f64),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<JsValue>),
    /// An object, its members in the order they were written.
    Object(JsObject),
}

/// The tag of a number JSON has no text for, as the wire spells it; `None` for any other.
#[must_use]
pub fn number_tag(number: f64) -> Option<&'static str> {
    if number.is_nan() {
        return Some("NaN");
    }
    if number.is_infinite() {
        return Some(if number > 0.0 {
            "Infinity"
        } else {
            "-Infinity"
        });
    }
    (number == 0.0 && number.is_sign_negative()).then_some("-0")
}

/// The number a `$number` tag names.
fn tagged_number(name: &str) -> Option<f64> {
    match name {
        "NaN" => Some(f64::NAN),
        "Infinity" => Some(f64::INFINITY),
        "-Infinity" => Some(f64::NEG_INFINITY),
        "-0" => Some(-0.0),
        _ => None,
    }
}

impl JsValue {
    /// The value a wire tag stands for (`{"$undefined": true}`, `{"$number": name}`); `None` for
    /// members that are no such tag.
    #[must_use]
    pub fn from_tag(members: &JsObject) -> Option<Self> {
        if members.len() != 1 {
            return None;
        }
        match members.first() {
            Some((key, Self::Bool(true))) if key == "$undefined" => Some(Self::Undefined),
            Some((key, Self::String(name))) if key == "$number" => {
                tagged_number(name).map(Self::Number)
            }
            Some(_) | None => None,
        }
    }

    /// JavaScript's `typeof`: `null`, an array and an object are all `object`.
    #[must_use]
    pub const fn type_of(&self) -> &'static str {
        match self {
            Self::Undefined => "undefined",
            Self::Null | Self::Array(_) | Self::Object(_) => "object",
            Self::Bool(_) => "boolean",
            Self::Number(_) => "number",
            Self::String(_) => "string",
        }
    }

    /// Whether it is `undefined`.
    #[must_use]
    pub const fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }

    /// Whether it is `null`.
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// `value == null`: `undefined` or `null`.
    #[must_use]
    pub const fn is_nullish(&self) -> bool {
        matches!(self, Self::Undefined | Self::Null)
    }

    /// The boolean, when it is one.
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(flag) => Some(*flag),
            Self::Undefined
            | Self::Null
            | Self::Number(_)
            | Self::String(_)
            | Self::Array(_)
            | Self::Object(_) => None,
        }
    }

    /// The number, when it is one (`NaN` included).
    #[must_use]
    pub const fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(number) => Some(*number),
            Self::Undefined
            | Self::Null
            | Self::Bool(_)
            | Self::String(_)
            | Self::Array(_)
            | Self::Object(_) => None,
        }
    }

    /// The string, when it is one.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            Self::Undefined
            | Self::Null
            | Self::Bool(_)
            | Self::Number(_)
            | Self::Array(_)
            | Self::Object(_) => None,
        }
    }

    /// The items, when it is an array.
    #[must_use]
    pub const fn as_array(&self) -> Option<&Vec<Self>> {
        match self {
            Self::Array(items) => Some(items),
            Self::Undefined
            | Self::Null
            | Self::Bool(_)
            | Self::Number(_)
            | Self::String(_)
            | Self::Object(_) => None,
        }
    }

    /// The members, when it is an object (not `null`, not an array).
    #[must_use]
    pub const fn as_object(&self) -> Option<&JsObject> {
        match self {
            Self::Object(members) => Some(members),
            Self::Undefined
            | Self::Null
            | Self::Bool(_)
            | Self::Number(_)
            | Self::String(_)
            | Self::Array(_) => None,
        }
    }

    /// An object's own member `key`, as written (one set to `undefined` too); `None` when it is
    /// no object or has no such member.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Self> {
        self.as_object().and_then(|members| members.get(key))
    }

    /// Whether it is a string.
    #[must_use]
    pub const fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    /// Whether it is a boolean.
    #[must_use]
    pub const fn is_boolean(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    /// Whether it is a number.
    #[must_use]
    pub const fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    /// Whether it is an array.
    #[must_use]
    pub const fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    /// Whether it is an object (not `null`, not an array).
    #[must_use]
    pub const fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }
}

impl From<bool> for JsValue {
    fn from(flag: bool) -> Self {
        Self::Bool(flag)
    }
}

impl From<f64> for JsValue {
    fn from(number: f64) -> Self {
        Self::Number(number)
    }
}

impl From<&str> for JsValue {
    fn from(text: &str) -> Self {
        Self::String(text.to_owned())
    }
}

impl From<String> for JsValue {
    fn from(text: String) -> Self {
        Self::String(text)
    }
}

impl From<Vec<Self>> for JsValue {
    fn from(items: Vec<Self>) -> Self {
        Self::Array(items)
    }
}

impl From<JsObject> for JsValue {
    fn from(members: JsObject) -> Self {
        Self::Object(members)
    }
}

impl<const N: usize> From<[(&str, Self); N]> for JsValue {
    /// An object literal: `JsValue::from([("a", 1.0.into())])` is `{ a: 1 }`.
    fn from(members: [(&str, Self); N]) -> Self {
        Self::Object(
            members
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests;
