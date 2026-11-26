//! A double-quoted scalar's value (`doubleQuotedValue` of `compose/resolve-flow-scalar.js`, with
//! `foldNewline` and `parseCharCode`): its lines folded and its escapes read.

use super::is_blank;
use crate::yaml::compose::Ctx;
use crate::yaml::units::{Units, string, unit, units};

/// `escapeCodes`.
fn escape_code(code: u16) -> Option<char> {
    Some(match char::from_u32(u32::from(code))? {
        '0' => '\0',
        'a' => '\x07',
        'b' => '\x08',
        'e' => '\x1b',
        'f' => '\x0c',
        'n' => '\n',
        'r' => '\r',
        't' | '\t' => '\t',
        'v' => '\x0b',
        'N' => '\u{85}',
        '_' => '\u{a0}',
        'L' => '\u{2028}',
        'P' => '\u{2029}',
        character @ (' ' | '"' | '/' | '\\') => character,
        _ => return None,
    })
}

fn push_char(out: &mut Units, character: char) {
    let mut buffer = [0; 2];
    out.extend_from_slice(character.encode_utf16(&mut buffer));
}

/// `doubleQuotedValue`.
pub(super) fn double_quoted_value(ctx: &mut Ctx<'_>, offset: usize, source: &[u16]) -> Units {
    let is = |index: usize, character: char| source.get(index) == Some(&unit(character));
    let mut res = Units::new();
    let mut i = 1;
    while i + 1 < source.len() {
        let Some(&ch) = source.get(i) else { break };
        if ch == unit('\r') && is(i + 1, '\n') {
            i += 1;
            continue;
        }
        if ch == unit('\n') {
            let (fold, next) = fold_newline(source, i);
            res.extend(fold);
            i = next;
        } else if ch == unit('\\') {
            i += 1;
            let next = source.get(i).copied();
            if let Some(code) = next.and_then(escape_code) {
                push_char(&mut res, code);
            } else if next == Some(unit('\n')) {
                // Skip escaped newlines, but still trim the following line.
                while source.get(i + 1).is_some_and(|code| is_blank(*code)) {
                    i += 1;
                }
            } else if next == Some(unit('\r')) && is(i + 1, '\n') {
                // Skip escaped CRLF newlines, but still trim the following line.
                i += 1;
                while source.get(i + 1).is_some_and(|code| is_blank(*code)) {
                    i += 1;
                }
            } else if let Some(length) = next.and_then(hex_length) {
                res.extend(parse_char_code(ctx, offset, source, i + 1, length));
                i += length;
            } else {
                let raw = source
                    .get(i - 1..(i + 1).min(source.len()))
                    .unwrap_or_default();
                ctx.error(
                    offset + i - 1,
                    format!("Invalid escape sequence {}", string(raw)),
                );
                res.extend_from_slice(raw);
            }
        } else if is_blank(ch) {
            // Trim trailing whitespace.
            let ws_start = i;
            while source.get(i + 1).is_some_and(|code| is_blank(*code)) {
                i += 1;
            }
            let next = source.get(i + 1).copied();
            if next != Some(unit('\n')) && !(next == Some(unit('\r')) && is(i + 2, '\n')) {
                res.extend_from_slice(source.get(ws_start..=i).unwrap_or_default());
            }
        } else {
            res.push(ch);
        }
        i += 1;
    }
    if source.last() != Some(&unit('"')) || source.len() == 1 {
        ctx.error(offset + source.len(), "Missing closing \"quote");
    }
    res
}

/// The digits after `\x`, `\u` and `\U`.
fn hex_length(code: u16) -> Option<usize> {
    match char::from_u32(u32::from(code))? {
        'x' => Some(2),
        'u' => Some(4),
        'U' => Some(8),
        _ => None,
    }
}

/// `foldNewline`: a single line break folds to a space, more to one break less; and where the
/// folded run ends.
fn fold_newline(source: &[u16], mut offset: usize) -> (Units, usize) {
    let mut fold = Units::new();
    while let Some(&ch) = source.get(offset + 1) {
        if !(is_blank(ch) || ch == unit('\n') || ch == unit('\r')) {
            break;
        }
        if ch == unit('\r') && source.get(offset + 2) != Some(&unit('\n')) {
            break;
        }
        if ch == unit('\n') {
            fold.push(unit('\n'));
        }
        offset += 1;
    }
    if fold.is_empty() {
        fold.push(unit(' '));
    }
    (fold, offset)
}

/// `parseCharCode`: the character `\xXX`, `\uXXXX` or `\UXXXXXXXX` spells, or the escape kept
/// with an error when it spells none (`String.fromCodePoint` throws).
fn parse_char_code(
    ctx: &mut Ctx<'_>,
    base: usize,
    source: &[u16],
    offset: usize,
    length: usize,
) -> Units {
    let digits = source
        .get(offset..(offset + length).min(source.len()))
        .unwrap_or_default();
    let text = string(digits);
    let ok = digits.len() == length && text.bytes().all(|byte| byte.is_ascii_hexdigit());
    let code = if ok {
        u32::from_str_radix(&text, 16).ok()
    } else {
        None
    };
    match code {
        Some(code @ 0xD800..=0xDFFF) => u16::try_from(code)
            .map(|unit| vec![unit])
            .unwrap_or_default(),
        Some(code) if char::from_u32(code).is_some() => char::from_u32(code)
            .map(|character| units(&character.to_string()))
            .unwrap_or_default(),
        Some(_) | None => {
            let start = offset.saturating_sub(2);
            let raw = source
                .get(start..(offset + length).min(source.len()))
                .unwrap_or_default();
            ctx.error(
                base + start,
                format!("Invalid escape sequence {}", string(raw)),
            );
            raw.to_vec()
        }
    }
}
