//! `decodeURIComponent`, which the YAML reader applies to a tag's suffix (`!!str`, `!e!%21`), and
//! `encodeURIComponent`, which a `headersHelper`'s redaction looks for.

use std::fmt::Write;

/// The value of a hexadecimal digit.
fn hex_value(byte: u8) -> Option<u8> {
    char::from(byte)
        .to_digit(16)
        .and_then(|digit| u8::try_from(digit).ok())
}

/// The byte `%XX` at `at` spells, if it spells one.
fn escaped(bytes: &[u8], at: usize) -> Option<u8> {
    let high = hex_value(*bytes.get(at + 1)?)?;
    let low = hex_value(*bytes.get(at + 2)?)?;
    (bytes.get(at) == Some(&b'%')).then_some(high * 16 + low)
}

/// How many bytes a UTF-8 sequence that starts with `lead` has; `None` for no lead byte.
const fn sequence_length(lead: u8) -> Option<usize> {
    match lead {
        0x00..=0x7F => Some(1),
        0xC0..=0xDF => Some(2),
        0xE0..=0xEF => Some(3),
        0xF0..=0xF7 => Some(4),
        _ => None,
    }
}

/// `decodeURIComponent(text)`: every `%XX` escape decoded, a run of them as UTF-8. A malformed
/// escape or sequence (an overlong one, a surrogate, past U+10FFFF) is the error JavaScriptCore
/// throws, `URIError: URI error`, as `String(error)` writes it.
///
/// # Errors
///
/// The error's text when the escapes do not decode.
pub fn decode_uri_component(text: &str) -> Result<String, String> {
    const ERROR: &str = "URIError: URI error";
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        if byte != b'%' {
            out.push(byte);
            at += 1;
            continue;
        }
        let lead = escaped(bytes, at).ok_or_else(|| ERROR.to_owned())?;
        let length = sequence_length(lead).ok_or_else(|| ERROR.to_owned())?;
        let mut sequence = vec![lead];
        for index in 1..length {
            let next = escaped(bytes, at + 3 * index).ok_or_else(|| ERROR.to_owned())?;
            sequence.push(next);
        }
        let decoded = std::str::from_utf8(&sequence)
            .ok()
            .ok_or_else(|| ERROR.to_owned())?;
        out.extend_from_slice(decoded.as_bytes());
        at += 3 * length;
    }
    String::from_utf8(out).ok().ok_or_else(|| ERROR.to_owned())
}

/// `encodeURIComponent(text)`: every character but the letters, the digits and `-_.!~*'()` as the
/// `%XX` escapes of its UTF-8 bytes, in capitals. (A lone surrogate, where JavaScript throws, is
/// never in a Rust string.)
#[must_use]
pub fn encode_uri_component(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{decode_uri_component, encode_uri_component};

    /// What Bun answers: `bun -e "for (const s of ['abc','a b/c','-_.!~*\'()','é€😀',
    /// '%?#&=+$,;:@[]']) console.log(encodeURIComponent(s))"`.
    #[test]
    fn encodes_as_javascript_does() {
        assert_eq!(encode_uri_component("abc"), "abc");
        assert_eq!(encode_uri_component("a b/c"), "a%20b%2Fc");
        assert_eq!(encode_uri_component("-_.!~*'()"), "-_.!~*'()");
        assert_eq!(encode_uri_component("é€😀"), "%C3%A9%E2%82%AC%F0%9F%98%80");
        assert_eq!(
            encode_uri_component("%?#&=+$,;:@[]"),
            "%25%3F%23%26%3D%2B%24%2C%3B%3A%40%5B%5D"
        );
    }

    /// What Bun answers: `bun -e "for (const s of ['str','%21','%zz','%E2%82%AC','%C0%80',
    /// '%ED%A0%80','a%','%F4%90%80%80','%C3']) { try { console.log(decodeURIComponent(s)) }
    /// catch (e) { console.log(String(e)) } }"`.
    #[test]
    fn decodes_as_javascript_does() {
        let error = Err("URIError: URI error".to_owned());
        assert_eq!(decode_uri_component("str"), Ok("str".to_owned()));
        assert_eq!(decode_uri_component("%21"), Ok("!".to_owned()));
        assert_eq!(decode_uri_component("%E2%82%AC"), Ok("€".to_owned()));
        for bad in ["%zz", "%C0%80", "%ED%A0%80", "a%", "%F4%90%80%80", "%C3"] {
            assert_eq!(decode_uri_component(bad), error, "{bad}");
        }
    }
}
