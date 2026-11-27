//! The scans that tell which reason JavaScriptCore gives where the engine's error stands for
//! several: the first atom a quantifier or bracket cannot follow, and the structure of the
//! pattern (groups, classes, a trailing backslash).

/// How many more groups open than close, outside classes and escapes.
pub(super) fn depth(source: &[char]) -> i64 {
    let (mut depth, mut in_class, mut escaped) = (0i64, false, false);
    for character in source {
        match (escaped, in_class, character) {
            (true, _, _) => escaped = false,
            (false, _, '\\') => escaped = true,
            (false, false, '[') => in_class = true,
            (false, true, ']') => in_class = false,
            (false, false, '(') => depth += 1,
            (false, false, ')') => depth -= 1,
            _ => {}
        }
    }
    depth
}

/// A braced quantifier at `at` (`{n}`, `{n,}`, `{n,m}`): its length and whether `n > m`.
fn braced(source: &[char], at: usize) -> Option<(usize, bool)> {
    let rest = source.get(at + 1..)?;
    let close = rest.iter().position(|character| *character == '}')?;
    let inside: String = rest.get(..close)?.iter().collect();
    let (low, high) = inside.split_once(',').unwrap_or((&inside, &inside));
    let digits = |text: &str| !text.is_empty() && text.chars().all(|c| c.is_ascii_digit());
    if !digits(low) || !(high.is_empty() || digits(high)) {
        return None;
    }
    let reversed = !high.is_empty() && low.parse::<f64>().ok() > high.parse::<f64>().ok();
    Some((close + 2, reversed))
}

pub(super) fn braces_out_of_order(source: &[char]) -> bool {
    (0..source.len()).any(|at| {
        source.get(at) == Some(&'{') && braced(source, at).is_some_and(|(_, reversed)| reversed)
    })
}

/// What JavaScriptCore names where the engine met a character that cannot start an atom: a
/// quantifier with nothing before it or after a lookbehind, and with `u` a brace or bracket that
/// opens or closes nothing.
pub(super) fn atom_reason(source: &[char], unicode: bool) -> &'static str {
    let (mut at, mut quantifiable) = (0, false);
    let (mut lookbehinds, mut after_lookbehind) = (Vec::new(), false);
    while let Some(character) = source.get(at) {
        let mut next = at + 1;
        let closed_lookbehind = after_lookbehind;
        after_lookbehind = false;
        match character {
            '*' | '+' | '?' | '{' if closed_lookbehind => return "invalid quantifier",
            '\\' => {
                quantifiable = !matches!(source.get(at + 1), Some('b' | 'B'));
                next = at + 2;
                if matches!(source.get(at + 1), Some('u' | 'p' | 'P'))
                    && source.get(at + 2) == Some(&'{')
                {
                    let close = source
                        .get(at..)
                        .and_then(|rest| rest.iter().position(|c| *c == '}'));
                    next = close.map_or(next, |close| at + close + 1);
                }
            }
            '[' => {
                next = class_end(source, at);
                quantifiable = true;
            }
            '(' => {
                quantifiable = false;
                lookbehinds.push(
                    source.get(at + 1..at + 3) == Some(&['?', '<'])
                        && group_header(source, at) == 4,
                );
                next = at + group_header(source, at);
            }
            ')' => {
                quantifiable = true;
                after_lookbehind = lookbehinds.pop().unwrap_or_default();
            }
            '|' | '^' | '$' => quantifiable = false,
            '*' | '+' | '?' if !quantifiable => return "nothing to repeat",
            '*' | '+' | '?' => {
                quantifiable = false;
                next = at
                    + if source.get(at + 1) == Some(&'?') {
                        2
                    } else {
                        1
                    };
            }
            '{' => match braced(source, at) {
                Some(_) if !quantifiable => return "nothing to repeat",
                Some((length, _)) => {
                    quantifiable = false;
                    next = at + length + usize::from(source.get(at + length) == Some(&'?'));
                }
                None if unicode => return "incomplete {} quantifier for Unicode pattern",
                None => quantifiable = true,
            },
            '}' | ']' if unicode => return "unmatched ] or } bracket for Unicode pattern",
            _ => quantifiable = true,
        }
        at = next;
    }
    "nothing to repeat"
}

/// The length of the group header at `at`: `(`, `(?:`, `(?=`, `(?!`, `(?<=`, `(?<!` or `(?<name>`.
pub(super) fn group_header(source: &[char], at: usize) -> usize {
    let after = |offset: usize| source.get(at + offset).copied();
    match (after(1), after(2), after(3)) {
        (Some('?'), Some(':' | '=' | '!'), _) | (Some('?'), Some('<'), Some('=' | '!')) => {
            if after(2) == Some('<') { 4 } else { 3 }
        }
        (Some('?'), Some('<'), _) => source
            .get(at..)
            .and_then(|rest| rest.iter().position(|character| *character == '>'))
            .map_or(1, |close| close + 1),
        _ => 1,
    }
}

/// Where the class that starts at `at` ends (after its `]`).
fn class_end(source: &[char], at: usize) -> usize {
    let mut index = at + 1;
    while let Some(character) = source.get(index) {
        match character {
            '\\' => index += 2,
            ']' => return index + 1,
            _ => index += 1,
        }
    }
    index
}

/// The error JavaScriptCore meets in reading `source` before it checks the names it refers to.
pub(super) fn structural(source: &[char], unicode: bool) -> Option<&'static str> {
    let atom = atom_reason(source, unicode);
    if atom != "nothing to repeat" {
        return Some(atom);
    }
    let backslashes = source
        .iter()
        .rev()
        .take_while(|character| **character == '\\')
        .count();
    if backslashes % 2 == 1 {
        return Some("\\ at end of pattern");
    }
    let (mut at, mut in_class) = (0, false);
    while let Some(character) = source.get(at) {
        match character {
            '\\' => at += 1,
            '[' => in_class = true,
            ']' => in_class = false,
            _ => {}
        }
        at += 1;
    }
    if in_class {
        return Some("missing terminating ] for character class");
    }
    match depth(source) {
        1.. => Some("missing )"),
        ..0 => Some("unmatched parentheses"),
        0 => None,
    }
}
