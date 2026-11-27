//! The YAML 1.2 core schema of `yaml` (`dist/schema/core/`, `dist/schema/common/`): its tags in
//! the package's order, each with the test a plain scalar is resolved by, how its text resolves,
//! which values it identifies, and how it writes one.

use std::sync::LazyLock;

use regex::Regex;

use super::directives::DEFAULT_TAG_PREFIX;
use super::nodes::{Format, Scalar};
use crate::js::{self, JsValue, Pattern, regex_test};

/// A tag of the core schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tag {
    Map,
    Seq,
    Str,
    Null,
    Bool,
    IntOct,
    Int,
    IntHex,
    FloatNaN,
    FloatExp,
    Float,
}

/// The core schema's tags, in the package's order (`schema/core/schema.js`).
pub(crate) const CORE: [Tag; 11] = [
    Tag::Map,
    Tag::Seq,
    Tag::Str,
    Tag::Null,
    Tag::Bool,
    Tag::IntOct,
    Tag::Int,
    Tag::IntHex,
    Tag::FloatNaN,
    Tag::FloatExp,
    Tag::Float,
];

fn pattern(source: &str) -> Option<Regex> {
    Regex::new(source).ok()
}

static NULL: Pattern = LazyLock::new(|| pattern(r"^(?:~|[Nn]ull|NULL)?$"));
static BOOL: Pattern = LazyLock::new(|| pattern(r"^(?:[Tt]rue|TRUE|[Ff]alse|FALSE)$"));
static INT_OCT: Pattern = LazyLock::new(|| pattern(r"^0o[0-7]+$"));
static INT: Pattern = LazyLock::new(|| pattern(r"^[-+]?[0-9]+$"));
static INT_HEX: Pattern = LazyLock::new(|| pattern(r"^0x[0-9a-fA-F]+$"));
static FLOAT_NAN: Pattern =
    LazyLock::new(|| pattern(r"^(?:[-+]?\.(?:inf|Inf|INF)|\.nan|\.NaN|\.NAN)$"));
static FLOAT_EXP: Pattern =
    LazyLock::new(|| pattern(r"^[-+]?(?:\.[0-9]+|[0-9]+(?:\.[0-9]*)?)[eE][-+]?[0-9]+$"));
static FLOAT: Pattern = LazyLock::new(|| pattern(r"^[-+]?(?:\.[0-9]+|[0-9]+\.[0-9]*)$"));

/// `Number.isInteger`.
fn is_integer(value: &JsValue) -> bool {
    value
        .as_f64()
        .is_some_and(|number| number.is_finite() && number.fract() == 0.0)
}

/// JavaScript's `Number(value)` of a scalar's value.
pub(crate) fn to_number(value: &JsValue) -> f64 {
    match value {
        JsValue::Number(number) => *number,
        JsValue::Bool(flag) => f64::from(u8::from(*flag)),
        JsValue::Null => 0.0,
        JsValue::String(text) => js::number(text),
        JsValue::Undefined | JsValue::Array(_) | JsValue::Object(_) => f64::NAN,
    }
}

/// A scalar as a tag's `resolve` gives it.
fn resolved(value: JsValue) -> Scalar {
    Scalar {
        value,
        source: None,
        kind: None,
        format: None,
        min_fraction_digits: 0,
    }
}

impl Tag {
    /// The tag's name.
    pub(crate) const fn tag(self) -> &'static str {
        match self {
            Self::Map => "tag:yaml.org,2002:map",
            Self::Seq => "tag:yaml.org,2002:seq",
            Self::Str => "tag:yaml.org,2002:str",
            Self::Null => "tag:yaml.org,2002:null",
            Self::Bool => "tag:yaml.org,2002:bool",
            Self::IntOct | Self::Int | Self::IntHex => "tag:yaml.org,2002:int",
            Self::FloatNaN | Self::FloatExp | Self::Float => "tag:yaml.org,2002:float",
        }
    }

    pub(crate) const fn format(self) -> Option<Format> {
        match self {
            Self::IntOct => Some(Format::Oct),
            Self::IntHex => Some(Format::Hex),
            Self::FloatExp => Some(Format::Exp),
            Self::Map
            | Self::Seq
            | Self::Str
            | Self::Null
            | Self::Bool
            | Self::Int
            | Self::FloatNaN
            | Self::Float => None,
        }
    }

    /// The collection a collection tag is for.
    pub(crate) const fn collection(self) -> Option<&'static str> {
        match self {
            Self::Map => Some("map"),
            Self::Seq => Some("seq"),
            Self::Str
            | Self::Null
            | Self::Bool
            | Self::IntOct
            | Self::Int
            | Self::IntHex
            | Self::FloatNaN
            | Self::FloatExp
            | Self::Float => None,
        }
    }

    fn test_pattern(self) -> Option<&'static Pattern> {
        match self {
            Self::Map | Self::Seq | Self::Str => None,
            Self::Null => Some(&NULL),
            Self::Bool => Some(&BOOL),
            Self::IntOct => Some(&INT_OCT),
            Self::Int => Some(&INT),
            Self::IntHex => Some(&INT_HEX),
            Self::FloatNaN => Some(&FLOAT_NAN),
            Self::FloatExp => Some(&FLOAT_EXP),
            Self::Float => Some(&FLOAT),
        }
    }

    /// Whether the tag has a `test`.
    pub(crate) fn has_test(self) -> bool {
        self.test_pattern().is_some()
    }

    /// `tag.test?.test(text)`.
    pub(crate) fn test(self, text: &str) -> bool {
        self.test_pattern()
            .is_some_and(|pattern| regex_test(pattern, text))
    }

    /// `tag.identify?.(value)`: whether the tag is the one for a value written from data.
    pub(crate) fn identify(self, value: &JsValue) -> bool {
        match self {
            Self::Map | Self::Seq => false,
            Self::Str => value.is_string(),
            Self::Null => value.is_nullish(),
            Self::Bool => value.is_boolean(),
            Self::IntOct | Self::IntHex => {
                is_integer(value) && value.as_f64().is_some_and(|number| number >= 0.0)
            }
            Self::Int => is_integer(value),
            Self::FloatNaN | Self::FloatExp | Self::Float => value.is_number(),
        }
    }

    /// `tag.resolve(text)` for a scalar tag.
    pub(crate) fn resolve(self, text: &str) -> Scalar {
        match self {
            Self::Map | Self::Seq | Self::Str => resolved(JsValue::String(text.to_owned())),
            Self::Null => resolved(JsValue::Null),
            Self::Bool => resolved(JsValue::Bool(text.starts_with(['t', 'T']))),
            Self::IntOct => resolved(JsValue::Number(js::parse_int_octal(
                text.get(2..).unwrap_or_default(),
            ))),
            Self::Int => resolved(JsValue::Number(js::number(text))),
            Self::IntHex => resolved(JsValue::Number(js::parse_int_hex(
                text.get(2..).unwrap_or_default(),
            ))),
            Self::FloatNaN => resolved(JsValue::Number(if text.to_lowercase().ends_with("nan") {
                f64::NAN
            } else if text.starts_with('-') {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            })),
            Self::FloatExp => resolved(JsValue::Number(js::number(text))),
            Self::Float => {
                let mut scalar = resolved(JsValue::Number(js::number(text)));
                if let Some(dot) = text.find('.')
                    && text.ends_with('0')
                {
                    scalar.min_fraction_digits = text.len() - dot - 1;
                }
                scalar
            }
        }
    }
}

/// `stringifyNumber`: a number's text, `.nan` and the infinities YAML's, decimals kept to the
/// count it was read with.
pub(crate) fn stringify_number(scalar: &Scalar, tag: Option<&str>) -> String {
    let number = to_number(&scalar.value);
    if !number.is_finite() {
        return if number.is_nan() {
            ".nan"
        } else if number < 0.0 {
            "-.inf"
        } else {
            ".inf"
        }
        .to_owned();
    }
    let negative_zero =
        matches!(scalar.value, JsValue::Number(zero) if zero == 0.0 && zero.is_sign_negative());
    let mut text = if negative_zero {
        "-0".to_owned()
    } else {
        scalar.value.to_json_text()
    };
    let float_tag = format!("{DEFAULT_TAG_PREFIX}float");
    let starts_with_digit = text
        .strip_prefix('-')
        .unwrap_or(&text)
        .starts_with(|character: char| character.is_ascii_digit());
    if scalar.format.is_none()
        && scalar.min_fraction_digits > 0
        && tag.is_none_or(|tag| tag == float_tag)
        && starts_with_digit
        && !text.contains('e')
    {
        let point = match text.find('.') {
            Some(point) => point,
            None => {
                text.push('.');
                text.len() - 1
            }
        };
        let decimals = text.len() - point - 1;
        for _ in decimals..scalar.min_fraction_digits {
            text.push('0');
        }
    }
    text
}

/// `intStringify`: an integer in a radix with its prefix, any other value as a number.
pub(crate) fn stringify_int(
    scalar: &Scalar,
    tag: Option<&str>,
    radix: u32,
    prefix: &str,
) -> String {
    match scalar.value.as_f64() {
        Some(number) if is_integer(&scalar.value) && number >= 0.0 => {
            format!("{prefix}{}", js::to_radix_string(number, radix))
        }
        Some(_) | None => stringify_number(scalar, tag),
    }
}
