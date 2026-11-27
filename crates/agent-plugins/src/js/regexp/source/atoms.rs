//! The escapes of a pattern without `u`, read as ECMAScript's Annex B reads them: what each one
//! stands for, and how many code units it spans.

/// What an escape outside a class stands for.
pub(super) enum Escape {
    /// One code unit, matched as a literal.
    Unit(u32, usize),
    /// A class escape, an assertion or a control character: copied as written.
    Verbatim(usize),
    /// A backreference, by number or by name.
    Backreference(usize),
    /// `\c` without a letter after it: the backslash is a literal, and `c` is read next.
    Backslash,
}

/// What a class member stands for.
#[derive(Clone, Copy)]
pub(super) enum ClassAtom {
    /// One code unit.
    Unit(u32, usize),
    /// A class escape (`\d`, `\W`…), copied as written.
    Set(usize),
}

/// Whether `unit` is an ASCII letter.
pub(super) fn is_letter(unit: u32) -> bool {
    char::from_u32(unit).is_some_and(|character| character.is_ascii_alphabetic())
}

fn digit(unit: Option<&u32>, radix: u32) -> Option<u32> {
    char::from_u32(*unit?)?.to_digit(radix)
}

/// The value of `count` hexadecimal digits at `at`.
fn hex(units: &[u32], at: usize, count: usize) -> Option<u32> {
    (0..count).try_fold(0, |value, offset| {
        Some(value * 16 + digit(units.get(at + offset), 16)?)
    })
}

/// A legacy octal escape at `at` (its first digit): up to three digits from `0`–`3`, two from
/// `4`–`7`, and its length.
fn octal(units: &[u32], at: usize) -> (u32, usize) {
    let first = digit(units.get(at), 8).unwrap_or_default();
    let most = if first <= 3 { 3 } else { 2 };
    let mut value = first;
    let mut length = 1;
    while length < most {
        let Some(next) = digit(units.get(at + length), 8) else {
            break;
        };
        value = value * 8 + next;
        length += 1;
    }
    (value, length)
}

/// The control escapes `\f \n \r \t \v`.
fn control(letter: char) -> Option<u32> {
    match letter {
        'f' => Some(0x0C),
        'n' => Some(0x0A),
        'r' => Some(0x0D),
        't' => Some(0x09),
        'v' => Some(0x0B),
        _ => None,
    }
}

/// `\x` and `\u` with their digits, else the letter itself; with the escape's length.
fn hex_escape(units: &[u32], at: usize, letter: u32, count: usize) -> (u32, usize) {
    hex(units, at + 2, count).map_or((letter, 2), |value| (value, 2 + count))
}

/// The escape at `at` (a backslash) outside a class, given how many capturing groups the pattern
/// has and whether one is named.
pub(super) fn escape(units: &[u32], at: usize, groups: usize, named: bool) -> Escape {
    let Some(&letter) = units.get(at + 1) else {
        return Escape::Verbatim(1);
    };
    let Some(character) = char::from_u32(letter) else {
        return Escape::Unit(letter, 2);
    };
    match character {
        'b' | 'B' | 'd' | 'D' | 's' | 'S' | 'w' | 'W' | 'f' | 'n' | 'r' | 't' | 'v' => {
            Escape::Verbatim(2)
        }
        'c' if units.get(at + 2).copied().is_some_and(is_letter) => Escape::Verbatim(3),
        'c' => Escape::Backslash,
        'k' if named => {
            let close = units
                .get(at..)
                .and_then(|rest| rest.iter().position(|unit| *unit == u32::from('>')));
            Escape::Backreference(close.map_or(2, |close| close + 1))
        }
        '0' if digit(units.get(at + 2), 8).is_none() => Escape::Verbatim(2),
        '1'..='9' => decimal(units, at, groups),
        '0' => {
            let (value, length) = octal(units, at + 1);
            Escape::Unit(value, 1 + length)
        }
        'x' => {
            let (value, length) = hex_escape(units, at, letter, 2);
            Escape::Unit(value, length)
        }
        'u' => {
            let (value, length) = hex_escape(units, at, letter, 4);
            Escape::Unit(value, length)
        }
        _ => Escape::Unit(letter, 2),
    }
}

/// `\` and digits: a backreference when the number is a group's, else a legacy octal escape or,
/// for `8` and `9`, the digit itself.
fn decimal(units: &[u32], at: usize, groups: usize) -> Escape {
    let mut length = 1;
    let mut number: usize = 0;
    while let Some(value) = digit(units.get(at + length), 10) {
        number = number
            .saturating_mul(10)
            .saturating_add(usize::try_from(value).unwrap_or(usize::MAX));
        length += 1;
    }
    if number <= groups {
        return Escape::Backreference(length);
    }
    match units.get(at + 1).copied() {
        Some(first) if digit(Some(&first), 8).is_some() => {
            let (value, length) = octal(units, at + 1);
            Escape::Unit(value, 1 + length)
        }
        Some(first) => Escape::Unit(first, 2),
        None => Escape::Verbatim(1),
    }
}

/// The class member at `at`.
pub(super) fn class_atom(units: &[u32], at: usize) -> ClassAtom {
    let Some(&unit) = units.get(at) else {
        return ClassAtom::Set(0);
    };
    if unit != u32::from('\\') {
        return ClassAtom::Unit(unit, 1);
    }
    let Some(&letter) = units.get(at + 1) else {
        return ClassAtom::Unit(unit, 1);
    };
    let Some(character) = char::from_u32(letter) else {
        return ClassAtom::Unit(letter, 2);
    };
    let after = units.get(at + 2).copied();
    match character {
        'd' | 'D' | 's' | 'S' | 'w' | 'W' => ClassAtom::Set(2),
        'b' => ClassAtom::Unit(0x08, 2),
        'c' if after.is_some_and(|next| {
            is_letter(next) || digit(Some(&next), 10).is_some() || next == u32::from('_')
        }) =>
        {
            ClassAtom::Unit(after.unwrap_or_default() % 32, 3)
        }
        'c' => ClassAtom::Unit(unit, 1),
        '0'..='7' => {
            let (value, length) = octal(units, at + 1);
            ClassAtom::Unit(value, 1 + length)
        }
        'x' => {
            let (value, length) = hex_escape(units, at, letter, 2);
            ClassAtom::Unit(value, length)
        }
        'u' => {
            let (value, length) = hex_escape(units, at, letter, 4);
            ClassAtom::Unit(value, length)
        }
        other => ClassAtom::Unit(control(other).unwrap_or(letter), 2),
    }
}
