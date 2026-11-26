//! What the npm package gets from JavaScript itself and Rust spells differently: strings are
//! UTF-16 code units there (their length, their order), a number has one text and reads from a
//! text by JavaScript's grammar, `JSON.parse` words its errors, `new URL` reads an address,
//! `normalize` composes a text and `new RegExp` reads a pattern by ECMAScript's grammar and
//! matches it on UTF-16 (`regexp`).
//! The ports of its modules use these wherever JavaScript's own semantics reach a result;
//! `scripts/rust/js-oracle.ts` records the built-ins themselves, which `tests/differential/js.rs`
//! holds these to.

pub mod array_join;
pub mod indexing;
pub mod json_parse;
pub mod json_stringify;
mod math;
pub mod normalize;
mod promise;
mod radix;
pub mod regexp;
mod slice;
mod string_to_number;
mod uri;
pub mod url;
pub mod value;

pub use array_join::{array_join, string_of};
pub use json_stringify::{json_stringify, json_stringify_indented};
pub use math::math_min;
pub use promise::{promise_all, promise_all_pair};
pub use radix::{to_exponential, to_radix_string};
pub use slice::{slice_code_units, slice_index};
pub use string_to_number::number;
pub use uri::decode_uri_component;
pub use value::{JsObject, JsValue};

use std::cmp::Ordering;
use std::sync::LazyLock;

use indexmap::IndexMap;
use regex::Regex;

/// The order of two texts by UTF-16 code units, JavaScript's `<` on strings. It differs from
/// Rust's byte order only between a character above U+FFFF and one in U+E000..=U+FFFF.
#[must_use]
pub fn compare_code_units(first: &str, second: &str) -> Ordering {
    first.encode_utf16().cmp(second.encode_utf16())
}

/// The length of a text in UTF-16 code units, JavaScript's `String.prototype.length`.
#[must_use]
pub fn code_unit_length(text: &str) -> usize {
    text.encode_utf16().count()
}

/// Whether a character is one JavaScript's `trim` removes: its `WhiteSpace` and
/// `LineTerminator`. Rust's `char::is_whitespace` differs in two: it keeps U+FEFF and removes
/// U+0085.
#[must_use]
pub fn is_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{9}' | '\u{A}' | '\u{B}' | '\u{C}' | '\u{D}' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// A text without the blanks around it, as JavaScript's `String.prototype.trim` cuts them.
#[must_use]
pub fn trim(text: &str) -> &str {
    text.trim_matches(is_whitespace)
}

/// JavaScript's `\s` inside a character class of a Rust regular expression, `is_whitespace`'s
/// set (RS6): `[{SPACE_CLASS}]` is `\s` and `[^{SPACE_CLASS}()]` is `[^\s()]`. Rust's own `\s`
/// differs in U+0085 and U+FEFF, as `char::is_whitespace` does.
pub const SPACE_CLASS: &str = r"\t\n\x0B\x0C\r \u{A0}\u{1680}\u{2000}-\u{200A}\u{2028}\u{2029}\u{202F}\u{205F}\u{3000}\u{FEFF}";

/// The largest integer a double holds exactly, `Number.MAX_SAFE_INTEGER` (2^53 - 1).
pub const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// `Number.isSafeInteger`: a whole number a double holds exactly, so within 2^53 - 1 either way.
#[must_use]
pub fn is_safe_integer(value: f64) -> bool {
    value.is_finite() && value.fract() == 0.0 && value.abs() <= MAX_SAFE_INTEGER
}

/// A number's text as JavaScript writes it (`String(n)`, `JSON.stringify(n)` for a finite one):
/// the shortest digits that read back as the same number, plain from 1e-6 up to 1e21 and in
/// exponent form outside it.
#[must_use]
pub fn number_to_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    let (digits, exponent) = shortest_digits(value.abs());
    format!("{sign}{}", place_point(&digits, exponent))
}

/// A number with a fixed count of decimals as JavaScript's `Number.prototype.toFixed` writes it:
/// the exact value of the double rounded half up (Rust's `{:.n}` breaks a tie to even, so
/// `16.25` is `16.3` here and `16.2` there). From 1e21 on it is `number_to_string`'s text.
#[must_use]
pub fn to_fixed(value: f64, digits: usize) -> String {
    if value.is_nan() || value.abs() >= 1e21 {
        return number_to_string(value);
    }
    // Thirty places past the wanted ones hold the exact tail for every tie and near-tie.
    let text = format!("{:.*}", digits + 30, value.abs());
    let (whole, fraction) = text.split_once('.').unwrap_or((text.as_str(), ""));
    let mut kept: Vec<u8> = whole.bytes().chain(fraction.bytes().take(digits)).collect();
    let mut carry = fraction.as_bytes().get(digits).is_some_and(|d| *d >= b'5');
    for digit in kept.iter_mut().rev() {
        if !carry {
            break;
        }
        carry = *digit == b'9';
        *digit = if carry { b'0' } else { *digit + 1 };
    }
    if carry {
        kept.insert(0, b'1');
    }
    let point = kept.len().saturating_sub(digits);
    let (integer, decimals) = kept.split_at(point);
    let sign = if value < 0.0 { "-" } else { "" };
    let integer: String = integer.iter().map(|byte| char::from(*byte)).collect();
    if digits == 0 {
        return format!("{sign}{integer}");
    }
    let decimals: String = decimals.iter().map(|byte| char::from(*byte)).collect();
    format!("{sign}{integer}.{decimals}")
}

/// A regular expression literal of the package, compiled once. It is an `Option` because a
/// pattern that fails to compile must never panic (RS4); a test of the module catches it.
pub type Pattern = LazyLock<Option<Regex>>;

/// JavaScript's `regex.test(text)` for a [`Pattern`]. The pattern is written with JavaScript's
/// classes (RS6): `[0-9]`, `[A-Za-z0-9_]`, `(?-u:\b)`; `.` is `[^\n\r\u{2028}\u{2029}]`; a flag
/// `i` without `u` folds ASCII letters only, which `(?i-u:...)` spells.
#[must_use]
pub fn regex_test(pattern: &Pattern, text: &str) -> bool {
    pattern.as_ref().is_some_and(|regex| regex.is_match(text))
}

/// JavaScript's `ToInt32` of a number, what its bitwise operators (`&`, `>>`) apply first: the
/// integer part modulo 2^32 read as signed. NaN and the infinities are 0.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the value is an integer in -2^31..2^31 after the modulo"
)]
pub fn to_int32(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    let wrapped = value.trunc().rem_euclid(4_294_967_296.0);
    let signed = if wrapped >= 2_147_483_648.0 {
        wrapped - 4_294_967_296.0
    } else {
        wrapped
    };
    signed as i32
}

/// JavaScript's `Number.parseInt(text, 16)`: the hexadecimal digits the text starts with
/// (after blanks, with an optional sign and `0x`), or NaN when it starts with none.
#[must_use]
pub fn parse_int_hex(text: &str) -> f64 {
    let trimmed = trim_start(text);
    let (negative, unsigned) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let digits = unsigned
        .strip_prefix("0x")
        .or_else(|| unsigned.strip_prefix("0X"))
        .unwrap_or(unsigned);
    let run: Vec<u32> = digits
        .chars()
        .map_while(|character| character.to_digit(16))
        .collect();
    if run.is_empty() {
        return f64::NAN;
    }
    let number = radix_value(&run, 16.0);
    if negative { -number } else { number }
}

/// The value of digits in a power-of-two radix as the runtime's `parseInt` sums them
/// (JavaScriptCore): digit by digit, most significant first, while it stays below 2^53; past that
/// the digits are summed again from the least significant one, each times a running power of the
/// radix that is itself a double, so a long run is not the correctly rounded number
/// (`parseIntOverflow`). Zeros after the power has overflowed add nothing; any other digit then
/// makes the value infinite.
fn radix_value(digits: &[u32], radix: f64) -> f64 {
    let mut number = 0.0_f64;
    for digit in digits {
        number = number * radix + f64::from(*digit);
    }
    if number < 9_007_199_254_740_992.0 {
        return number;
    }
    let mut sum = 0.0_f64;
    let mut multiplier = 1.0_f64;
    for digit in digits.iter().rev() {
        if multiplier.is_infinite() {
            if *digit != 0 {
                return f64::INFINITY;
            }
        } else {
            sum += f64::from(*digit) * multiplier;
        }
        multiplier *= radix;
    }
    sum
}

/// JavaScript's `Number.parseInt(digits, 8)` of a run of octal digits (as YAML's `0o17` holds
/// after its prefix), NaN for none.
#[must_use]
pub fn parse_int_octal(digits: &str) -> f64 {
    let run: Vec<u32> = digits
        .chars()
        .map_while(|character| character.to_digit(8))
        .collect();
    if run.is_empty() {
        return f64::NAN;
    }
    radix_value(&run, 8.0)
}

/// A text without the blanks before it, as JavaScript's `String.prototype.trimStart` cuts them.
#[must_use]
pub fn trim_start(text: &str) -> &str {
    text.trim_start_matches(is_whitespace)
}

/// `text.split(/\r?\n/)`: the lines of a text, each cut at a `\n` or a `\r\n`. A `\r` that is
/// not followed by a `\n` stays in its line, and so does one at the very end of the text.
#[must_use]
pub fn split_line_breaks(text: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = text.split('\n').collect();
    let last = lines.len().saturating_sub(1);
    for line in lines.iter_mut().take(last) {
        *line = line.strip_suffix('\r').unwrap_or(line);
    }
    lines
}

/// The shortest round-trip digits of a positive finite number and `n`, the position of the
/// decimal point after the first digit counted as JavaScript's algorithm counts it. Of the
/// shortest, the closest to the number, the even one of two as close (Number::toString, ECMA-262
/// §6.1.6.1.20): Rust's shortest digits settle the count, its exact rounding then the digits.
pub(crate) fn shortest_digits(value: f64) -> (String, i64) {
    let shortest = format!("{value:e}");
    let count = shortest
        .split('e')
        .next()
        .unwrap_or_default()
        .bytes()
        .filter(u8::is_ascii_digit)
        .count();
    let scientific = format!("{value:.*e}", count.saturating_sub(1));
    let (mantissa, exponent) = scientific
        .split_once('e')
        .unwrap_or((scientific.as_str(), "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let trimmed = digits.trim_end_matches('0');
    let digits = if trimmed.is_empty() { "0" } else { trimmed };
    (digits.to_owned(), exponent.parse::<i64>().unwrap_or(0) + 1)
}

fn place_point(digits: &str, n: i64) -> String {
    let k = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    if (k..=21).contains(&n) {
        return format!(
            "{digits}{}",
            "0".repeat(usize::try_from(n - k).unwrap_or(0))
        );
    }
    if (1..=21).contains(&n) {
        let (whole, fraction) = digits.split_at(usize::try_from(n).unwrap_or(0));
        return format!("{whole}.{fraction}");
    }
    if (-5..=0).contains(&n) {
        return format!("0.{}{digits}", "0".repeat(usize::try_from(-n).unwrap_or(0)));
    }
    let exponent = n - 1;
    let sign = if exponent < 0 { '-' } else { '+' };
    let (first, rest) = digits.split_at(1);
    let point = if rest.is_empty() {
        String::new()
    } else {
        format!(".{rest}")
    };
    format!("{first}{point}e{sign}{}", exponent.abs())
}

/// Whether a key is an array index, which a JavaScript object lists first: the canonical text of
/// an integer from 0 to 2^32 - 2.
fn array_index(key: &str) -> Option<u32> {
    let canonical =
        key == "0" || (!key.starts_with('0') && key.bytes().all(|b| b.is_ascii_digit()));
    if !canonical || key.is_empty() {
        return None;
    }
    key.parse::<u32>().ok().filter(|index| *index != u32::MAX)
}

/// Array-index keys ascending, then the others in the order they were written: the order of
/// `Object.keys`, `Object.entries`, `Object.values` and `for...in` over an object.
fn ordered<'a, T>(items: impl IntoIterator<Item = (&'a String, T)>) -> Vec<(&'a String, T)> {
    let mut indexed: Vec<(u32, (&String, T))> = Vec::new();
    let mut named: Vec<(&String, T)> = Vec::new();
    for item in items {
        match array_index(item.0) {
            Some(index) => indexed.push((index, item)),
            None => named.push(item),
        }
    }
    indexed.sort_by_key(|(index, _)| *index);
    indexed
        .into_iter()
        .map(|(_, item)| item)
        .chain(named)
        .collect()
}

/// The members of an object in the order `Object.entries` and `Object.keys` give them: the array
/// indexes ascending, then the other keys in the order they were written.
#[must_use]
pub fn property_order(members: &JsObject) -> Vec<(&String, &JsValue)> {
    ordered(members)
}

/// An object with its members in JavaScript's own order, the order every object JavaScript makes
/// holds them in: what `JSON.parse` and an object literal give, so a reader of the result
/// iterates as `Object.entries` does without sorting again.
#[must_use]
pub fn in_property_order(members: JsObject) -> JsObject {
    if !members.keys().any(|key| array_index(key).is_some()) {
        return members;
    }
    let mut indexed: Vec<(u32, (String, JsValue))> = Vec::new();
    let mut named: Vec<(String, JsValue)> = Vec::new();
    for (key, value) in members {
        match array_index(&key) {
            Some(index) => indexed.push((index, (key, value))),
            None => named.push((key, value)),
        }
    }
    indexed.sort_by_key(|(index, _)| *index);
    indexed
        .into_iter()
        .map(|(_, member)| member)
        .chain(named)
        .collect()
}

/// The keys of a record held as any ordered map (`IndexMap`, a struct's field list) in the order
/// `Object.keys` gives them, as [`property_order`] orders an object's.
#[must_use]
pub fn key_order<'a>(keys: impl IntoIterator<Item = &'a String>) -> Vec<&'a String> {
    ordered(keys.into_iter().map(|key| (key, ())))
        .into_iter()
        .map(|(key, ())| key)
        .collect()
}

/// A record held as any ordered map, with its entries in the order an object holds them: the array
/// indexes ascending, then the other keys as written (`key_order`). A port builds one with it
/// wherever the TypeScript builds an object with `Object.fromEntries` or a spread.
#[must_use]
pub fn in_key_order<V>(map: IndexMap<String, V>) -> IndexMap<String, V> {
    #[expect(
        clippy::needless_collect,
        reason = "the order borrows the map the loop below drains"
    )]
    let order: Vec<String> = key_order(map.keys()).into_iter().cloned().collect();
    let mut map = map;
    order
        .into_iter()
        .filter_map(|key| map.swap_remove(&key).map(|value| (key, value)))
        .collect()
}

/// Whether JavaScript reads `value` as true in a condition: not `undefined`, `null`, `false`, `0`,
/// `-0`, `NaN` or the empty string.
#[must_use]
pub fn is_truthy(value: &JsValue) -> bool {
    match value {
        JsValue::Undefined | JsValue::Null => false,
        JsValue::Bool(flag) => *flag,
        JsValue::Number(number) => *number != 0.0 && !number.is_nan(),
        JsValue::String(text) => !text.is_empty(),
        JsValue::Array(_) | JsValue::Object(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // bun -e 'console.log([undefined,null,false,0,-0,NaN,"","0",[],{}].map(Boolean))' prints
    // [ false, false, false, false, false, false, false, true, true, true ]
    #[test]
    fn truthiness_follows_javascript() {
        let falsy = [
            JsValue::Undefined,
            JsValue::Null,
            JsValue::Bool(false),
            JsValue::Number(0.0),
            JsValue::Number(-0.0),
            JsValue::Number(f64::NAN),
            JsValue::String(String::new()),
        ];
        assert!(falsy.iter().all(|value| !is_truthy(value)));
        let truthy = [
            JsValue::String("0".to_owned()),
            JsValue::Array(Vec::new()),
            JsValue::Object(JsObject::new()),
            JsValue::Number(1.0),
        ];
        assert!(truthy.iter().all(is_truthy));
    }

    // bun -e 'console.log(Object.keys({b:1,"10":2,a:3,"2":4}))' prints [ "2", "10", "b", "a" ]
    #[test]
    fn a_record_is_held_in_key_order() {
        let map = IndexMap::from([
            ("b".to_owned(), 1),
            ("10".to_owned(), 2),
            ("a".to_owned(), 3),
            ("2".to_owned(), 4),
        ]);
        let keys: Vec<String> = in_key_order(map).into_keys().collect();
        assert_eq!(keys, ["2", "10", "b", "a"]);
    }

    // bun -e 'console.log(JSON.stringify(["a\r\nb","a\r\r\nb\r","\n","", "a\rb"].map((t) => t.split(/\r?\n/))))' prints
    // [["a","b"],["a\r","b\r"],["",""],[""],["a\rb"]]
    #[test]
    fn lines_are_cut_at_a_line_feed_with_one_carriage_return() {
        assert_eq!(split_line_breaks("a\r\nb"), ["a", "b"]);
        assert_eq!(split_line_breaks("a\r\r\nb\r"), ["a\r", "b\r"]);
        assert_eq!(split_line_breaks("\n"), ["", ""]);
        assert_eq!(split_line_breaks(""), [""]);
        assert_eq!(split_line_breaks("a\rb"), ["a\rb"]);
    }

    #[test]
    fn the_space_class_is_javascripts_whitespace() {
        let class = Regex::new(&format!("^[{SPACE_CLASS}]$")).ok();
        assert!(class.is_some());
        for point in 0..=0x2FFFF_u32 {
            let Some(character) = char::from_u32(point) else {
                continue;
            };
            let text = character.to_string();
            let in_class = class.as_ref().is_some_and(|regex| regex.is_match(&text));
            assert_eq!(in_class, is_whitespace(character), "U+{point:04X}");
        }
    }

    #[test]
    fn numbers_read_as_javascript_writes_them() {
        let cases = [
            (1.0, "1"),
            (-0.0, "0"),
            (0.1, "0.1"),
            (123.456, "123.456"),
            (1e21, "1e+21"),
            (1e20, "100000000000000000000"),
            (1e-7, "1e-7"),
            (0.000_001, "0.000001"),
            (1.5e-7, "1.5e-7"),
            (2.5e25, "2.5e+25"),
            (900_719_925_474_099.2, "900719925474099.2"),
            (f64::NAN, "NaN"),
            (f64::NEG_INFINITY, "-Infinity"),
        ];
        for (value, text) in cases {
            assert_eq!(number_to_string(value), text, "{value:e}");
        }
    }

    #[test]
    fn fixed_rounds_half_up_on_the_exact_value() {
        let cases = [
            (16.25, 1, "16.3"),
            (16.2, 1, "16.2"),
            (0.5, 0, "1"),
            (2.5, 0, "3"),
            (9.96, 1, "10.0"),
            (0.0, 1, "0.0"),
            (-1.25, 1, "-1.3"),
            (1.005, 2, "1.00"),
            (123.456, 1, "123.5"),
        ];
        for (value, digits, text) in cases {
            assert_eq!(to_fixed(value, digits), text, "{value} {digits}");
        }
    }

    #[test]
    fn bitwise_and_hex_readings_follow_javascript() {
        assert_eq!(to_int32(4_294_967_295.0), -1);
        assert_eq!(to_int32(65_536.0 * 65_536.0 + 5.0), 5);
        assert_eq!(to_int32(f64::NAN), 0);
        assert_eq!(parse_int_hex("fe80"), 65_152.0);
        assert_eq!(parse_int_hex("1.2.3.4"), 1.0);
        assert_eq!(parse_int_hex("0x1f"), 31.0);
        assert!(parse_int_hex("").is_nan());
        assert!(parse_int_hex(".").is_nan());
        assert_eq!(parse_int_hex(" -a"), -10.0);
    }

    // bun -e 'for (const t of ["2744f72f879b898395472a", "270d8cb562dff97574edf942a0",
    //   "415c05406b0c2a775d08", "f".repeat(256), "1"+"0".repeat(256), "0".repeat(300)+"ff"])
    //   console.log(Number.parseInt(t, 16))'
    // JavaScriptCore sums a long run digit by digit, so it is not the correctly rounded number
    // (`Number(BigInt("0x2744f72f879b898395472a"))` is 4.747378765646893e+25).
    #[test]
    fn long_hex_runs_are_summed_as_the_runtime_sums_them() {
        assert_eq!(
            parse_int_hex("2744f72f879b898395472a"),
            4.747_378_765_646_894e25
        );
        assert_eq!(
            parse_int_hex("270d8cb562dff97574edf942a0"),
            3.094_091_749_370_986e30
        );
        assert_eq!(
            parse_int_hex("415c05406b0c2a775d08"),
            3.086_513_002_613_769_4e23
        );
        assert_eq!(parse_int_hex(&"f".repeat(256)), f64::INFINITY);
        assert_eq!(
            parse_int_hex(&format!("1{}", "0".repeat(256))),
            f64::INFINITY
        );
        assert_eq!(parse_int_hex(&format!("{}ff", "0".repeat(300))), 255.0);
    }

    // bun -e 'console.log(Object.keys({b:1, "10":1, "2":1, "01":1, a:1, "4294967295":1,
    //   "4294967294":1}))' lists 2, 10, 4294967294, b, 01, a, 4294967295.
    #[test]
    fn key_order_lists_indexes_first() {
        let keys: Vec<String> = ["b", "10", "2", "01", "a", "4294967295", "4294967294"]
            .iter()
            .map(|key| (*key).to_owned())
            .collect();
        let ordered: Vec<&str> = key_order(&keys).into_iter().map(String::as_str).collect();
        assert_eq!(
            ordered,
            ["2", "10", "4294967294", "b", "01", "a", "4294967295"]
        );
    }

    #[test]
    fn order_is_by_code_unit() {
        assert_eq!(
            compare_code_units("\u{FF61}", "\u{1F600}"),
            Ordering::Greater
        );
        assert_eq!("\u{FF61}".cmp("\u{1F600}"), Ordering::Less);
        assert_eq!(code_unit_length("a\u{1F600}"), 3);
        assert_eq!(trim("\u{FEFF} a \u{85}"), "a \u{85}");
    }

    #[test]
    fn the_space_class_is_is_whitespace() {
        let class = Regex::new(&format!("^[{SPACE_CLASS}]$")).ok();
        assert!(class.is_some(), "the class compiles");
        for character in (0..=0xFFFF_u32).filter_map(char::from_u32) {
            assert_eq!(
                class
                    .as_ref()
                    .is_some_and(|class| class.is_match(&character.to_string())),
                is_whitespace(character),
                "U+{:04X}",
                u32::from(character)
            );
        }
    }

    #[test]
    fn safe_integers_end_at_two_to_the_53_minus_1() {
        assert!(is_safe_integer(0.0));
        assert!(is_safe_integer(-0.0));
        assert!(is_safe_integer(MAX_SAFE_INTEGER));
        assert!(is_safe_integer(-MAX_SAFE_INTEGER));
        assert!(!is_safe_integer(MAX_SAFE_INTEGER + 1.0));
        assert!(!is_safe_integer(1.5));
        assert!(!is_safe_integer(f64::NAN));
        assert!(!is_safe_integer(f64::INFINITY));
    }

    #[test]
    fn keys_list_indexes_first() {
        let members: JsValue =
            json_parse::json_parse(r#"{"b":1,"10":2,"2":3,"01":4,"a":5}"#).unwrap_or_default();
        let keys: Vec<&str> = members
            .as_object()
            .map(property_order)
            .unwrap_or_default()
            .into_iter()
            .map(|(key, _)| key.as_str())
            .collect();
        assert_eq!(keys, ["2", "10", "b", "01", "a"]);
    }
}
