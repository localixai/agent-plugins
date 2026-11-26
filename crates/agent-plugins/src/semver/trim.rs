//! `internal/trim-comparator-whitespace.js`: the blank between an operator and its version
//! removed (`> 1.2.3` is `>1.2.3`), at the places the package's safe `COMPARATORTRIM`
//! replacement would remove it. Positions are UTF-16 code units and an end is exclusive, or its
//! start when nothing matched, as there.

use crate::js::utf16::{is_blank_unit, slice, text_of, unit_at, units};

use super::re::{MAX_LENGTH, MAX_SAFE_BUILD_LENGTH};

/// The last two runs of digits found, so their characters are not read again.
struct DigitRuns {
    start: Option<usize>,
    end: usize,
    previous_start: Option<usize>,
    previous_end: usize,
}

fn is(unit: Option<u16>, wanted: char) -> bool {
    unit.is_some_and(|unit| u32::from(unit) == u32::from(wanted))
}

fn is_digit(unit: Option<u16>) -> bool {
    unit.is_some_and(|unit| (0x30..=0x39).contains(&unit))
}

fn is_letter(unit: Option<u16>) -> bool {
    unit.is_some_and(|unit| (0x61..=0x7a).contains(&unit) || (0x41..=0x5a).contains(&unit))
}

fn is_identifier_character(unit: Option<u16>) -> bool {
    is_digit(unit) || is_letter(unit) || is(unit, '-')
}

/// `v`, `=` or a blank: a historical version prefix.
fn is_version_prefix(unit: Option<u16>) -> bool {
    is(unit, 'v') || is(unit, '=') || is_blank_unit(unit)
}

/// A digit or a wildcard: a version component can start here.
fn is_version_start(unit: Option<u16>) -> bool {
    is_digit(unit) || is(unit, 'x') || is(unit, 'X') || is(unit, '*')
}

/// `findDigitsEndIndex`: the end of a run of digits, no more than `limit` of them.
fn digits_end(range: &[u16], start: usize, limit: usize, runs: &mut DigitRuns) -> usize {
    if !is_digit(unit_at(range, start)) {
        return start;
    }
    if runs.start.is_some_and(|run| start >= run) && start < runs.end {
        return (start + limit).min(runs.end);
    }
    if runs.previous_start.is_some_and(|run| start >= run) && start < runs.previous_end {
        return (start + limit).min(runs.previous_end);
    }
    let mut end = start + 1;
    while is_digit(unit_at(range, end)) {
        end += 1;
    }
    // A failed full version can rewind from its second run to its first.
    runs.previous_start = runs.start;
    runs.previous_end = runs.end;
    runs.start = Some(start);
    runs.end = end;
    (start + limit).min(end)
}

/// `findVersionNumberEndIndex`: strict reads one `0`, loose any digits.
fn version_number_end(range: &[u16], start: usize, loose: bool, runs: &mut DigitRuns) -> usize {
    if !loose && is(unit_at(range, start), '0') {
        return start + 1;
    }
    let limit = if loose { MAX_LENGTH } else { MAX_LENGTH + 1 };
    digits_end(range, start, limit, runs)
}

/// `findPrereleaseIdentifierEndIndex`: the letter-or-hyphen form first, then the numeric one.
fn prerelease_identifier_end(
    range: &[u16],
    start: usize,
    loose: bool,
    runs: &mut DigitRuns,
) -> usize {
    let digits = digits_end(range, start, MAX_LENGTH, runs);
    if is_letter(unit_at(range, digits)) || is(unit_at(range, digits), '-') {
        let mut end = digits + 1;
        let limit = end + MAX_SAFE_BUILD_LENGTH;
        while end < limit && is_identifier_character(unit_at(range, end)) {
            end += 1;
        }
        return end;
    }
    version_number_end(range, start, loose, runs)
}

/// `findPrereleaseEndIndex`: `-beta.1`, one identifier at a time; loose allows no hyphen.
fn prerelease_end(range: &[u16], start: usize, loose: bool, runs: &mut DigitRuns) -> usize {
    let hyphen = is(unit_at(range, start), '-');
    if !loose && !hyphen {
        return start;
    }
    let identifier_start = start + usize::from(hyphen);
    let mut end = prerelease_identifier_end(range, identifier_start, loose, runs);
    if end == identifier_start {
        if !loose || !hyphen {
            return start;
        }
        // In loose syntax, a lone hyphen can itself be the identifier.
        end = prerelease_identifier_end(range, start, loose, runs);
    }
    while is(unit_at(range, end), '.') {
        let next = prerelease_identifier_end(range, end + 1, loose, runs);
        if next == end + 1 {
            break;
        }
        end = next;
    }
    end
}

/// `findBuildIdentifierEndIndex`.
fn build_identifier_end(range: &[u16], start: usize) -> usize {
    let mut end = start;
    while end - start < MAX_SAFE_BUILD_LENGTH && is_identifier_character(unit_at(range, end)) {
        end += 1;
    }
    end
}

/// `findBuildEndIndex`: `+sha.123`, or nothing.
fn build_end(range: &[u16], start: usize) -> usize {
    if !is(unit_at(range, start), '+') {
        return start;
    }
    let mut end = build_identifier_end(range, start + 1);
    if end == start + 1 {
        return start;
    }
    while is(unit_at(range, end), '.') {
        let next = build_identifier_end(range, end + 1);
        if next == end + 1 {
            break;
        }
        end = next;
    }
    end
}

/// `findLooseFullVersionEndIndex`: three numbers, then a pre-release and a build, loosely.
fn loose_full_version_end(range: &[u16], start: usize, runs: &mut DigitRuns) -> usize {
    let mut end = start;
    for component in 0..3 {
        let next = version_number_end(range, end, true, runs);
        if next == end {
            return start;
        }
        end = next;
        if component < 2 {
            if !is(unit_at(range, end), '.') {
                return start;
            }
            end += 1;
        }
    }
    let prerelease = prerelease_end(range, end, true, runs);
    build_end(range, prerelease)
}

/// `findPartialVersionComponentEndIndex`: a strict number, or one wildcard.
fn partial_component_end(range: &[u16], start: usize, runs: &mut DigitRuns) -> usize {
    if is_digit(unit_at(range, start)) {
        return version_number_end(range, start, false, runs);
    }
    if is_version_start(unit_at(range, start)) {
        start + 1
    } else {
        start
    }
}

/// `findPartialVersionEndIndex`: up to three components, then a strict pre-release and build.
fn partial_version_end(range: &[u16], start: usize, runs: &mut DigitRuns) -> usize {
    let mut end = partial_component_end(range, start, runs);
    for _ in 0..2 {
        if !is(unit_at(range, end), '.') {
            return end;
        }
        let next = partial_component_end(range, end + 1, runs);
        if next == end + 1 {
            return end;
        }
        end = next;
    }
    let prerelease = prerelease_end(range, end, false, runs);
    build_end(range, prerelease)
}

/// `findVersionEndIndex`: a loose full version, else a partial or wildcard one.
fn version_end(range: &[u16], start: usize, runs: &mut DigitRuns) -> usize {
    let full = loose_full_version_end(range, start, runs);
    if full == start {
        partial_version_end(range, start, runs)
    } else {
        full
    }
}

/// `findRemovableWhitespaceIndex`: the one blank after an optional operator, or `None`.
fn removable_whitespace(range: &[u16], match_start: usize) -> Option<usize> {
    let mut position = match_start;
    if is_blank_unit(unit_at(range, position)) {
        position += 1;
    }
    if is(unit_at(range, position), '<') || is(unit_at(range, position), '>') {
        position += 1;
    }
    if is(unit_at(range, position), '=') {
        position += 1;
    }
    is_blank_unit(unit_at(range, position)).then_some(position)
}

/// `trimComparatorWhitespace(range)`.
pub(super) fn trim_comparator_whitespace(text: &str) -> String {
    let range = units(text);
    let mut result: Vec<u16> = Vec::with_capacity(range.len());
    let mut copy_from = 0;
    let mut cursor = 0;
    let mut runs = DigitRuns {
        start: None,
        end: 0,
        previous_start: None,
        previous_end: 0,
    };
    while cursor < range.len() {
        let match_start = cursor;
        let here = unit_at(&range, cursor);
        if is(here, '<') || is(here, '>') {
            cursor += 1;
        } else if !is_version_prefix(here) && !is_version_start(here) {
            cursor += 1;
            continue;
        }
        // Consume a v/=/whitespace run once, even when no version follows it.
        while cursor < range.len() && is_version_prefix(unit_at(&range, cursor)) {
            cursor += 1;
        }
        if !is_version_start(unit_at(&range, cursor)) {
            continue;
        }
        if let Some(whitespace) = removable_whitespace(&range, match_start) {
            result.extend_from_slice(slice(&range, copy_from, whitespace));
            copy_from = whitespace + 1;
        }
        // Skip the entire match, so a pre-release's trailing v cannot be read as the prefix of
        // another version.
        cursor = version_end(&range, cursor, &mut runs);
    }
    result.extend_from_slice(slice(&range, copy_from, range.len()));
    text_of(&result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_an_operator_to_its_version() {
        assert_eq!(
            trim_comparator_whitespace("> 1.2.3 < 1.2.5"),
            ">1.2.3 <1.2.5"
        );
        assert_eq!(trim_comparator_whitespace(">= v1.2"), ">=v1.2");
        assert_eq!(trim_comparator_whitespace("1.2.3 - 2"), "1.2.3 - 2");
    }
}
