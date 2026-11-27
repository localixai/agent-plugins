//! One pass over a pattern's units that writes the engine's spelling of it (`source.rs`): groups
//! keep the state of `i` they open with, classes and escapes are read whole.

use std::ops::Range;

use super::super::legacy_case::{closure, is_cased};
use super::super::refusal::braced_length;
use super::atoms::{ClassAtom, Escape, class_atom, escape};
use super::{UNICODE_NOT_WORD, class_unit, is, is_group_name, modifiers};

/// Who folds case for the parts `i` applies to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Folding {
    /// The pass, by `Canonicalize`; the pattern is compiled without `i`.
    Pass,
    /// The engine, by its own folding.
    Engine,
}

/// The pass: the pattern, where it is, what it wrote, and the state of `i`.
pub(super) struct Scanner<'a> {
    units: &'a [u32],
    at: usize,
    out: Vec<u32>,
    icase: bool,
    /// Whether the pass folds case itself (`i` taken off the pattern), else leaves it to the engine.
    fold: Folding,
    scopes: Vec<bool>,
    groups: usize,
    named: bool,
    modified: bool,
}

/// How many capturing groups a pattern has, and whether one is named.
fn count_groups(units: &[u32]) -> (usize, bool) {
    let (mut groups, mut named, mut in_class, mut at) = (0, false, false, 0);
    while let Some(&unit) = units.get(at) {
        let character = char::from_u32(unit).unwrap_or_default();
        match (in_class, character) {
            (_, '\\') => at += 1,
            (false, '[') => in_class = true,
            (true, ']') => in_class = false,
            (false, '(') if !is(units.get(at + 1), '?') => groups += 1,
            (false, '(') if is(units.get(at + 2), '<') && is_group_name(units, at + 3) => {
                groups += 1;
                named = true;
            }
            _ => {}
        }
        at += 1;
    }
    (groups, named)
}

impl<'a> Scanner<'a> {
    pub(super) fn new(units: &'a [u32], icase: bool, fold: Folding) -> Self {
        let (groups, named) = count_groups(units);
        Self {
            units,
            at: 0,
            out: Vec::with_capacity(units.len()),
            icase,
            fold,
            scopes: Vec::new(),
            groups,
            named,
            modified: false,
        }
    }

    /// Whether the pass folds the unit it reads.
    fn folds(&self) -> bool {
        self.fold == Folding::Pass && self.icase
    }

    fn copy(&mut self, length: usize) {
        let end = (self.at + length).min(self.units.len());
        self.out
            .extend_from_slice(self.units.get(self.at..end).unwrap_or_default());
        self.at = end;
    }

    /// A group's opening: its header copied (a modifier group's without `i`), the state of `i`
    /// it was opened in kept for its end.
    fn open_group(&mut self, strip_icase: bool) {
        self.scopes.push(self.icase);
        if !is(self.units.get(self.at + 1), '?') {
            return self.copy(1);
        }
        let after = self.units.get(self.at + 2);
        if is(after, ':') || is(after, '=') || is(after, '!') {
            return self.copy(3);
        }
        if is(after, '<') {
            let close = self
                .units
                .get(self.at..)
                .and_then(|rest| rest.iter().position(|unit| *unit == u32::from('>')));
            let header = if is_group_name(self.units, self.at + 3) {
                close.map_or(3, |close| close + 1)
            } else {
                4
            };
            return self.copy(header);
        }
        let Some((inside, length, spelled)) = modifiers(self.units, self.at + 2, self.icase) else {
            return self.copy(2);
        };
        self.icase = inside;
        self.modified = true;
        if strip_icase {
            self.out.extend([u32::from('('), u32::from('?')]);
            self.out.extend(spelled);
            self.at += 2 + length;
        } else {
            self.copy(2 + length);
        }
    }

    fn close_group(&mut self) {
        self.icase = self.scopes.pop().unwrap_or(self.icase);
        self.copy(1);
    }

    /// A literal unit written at `span`: the class of what it matches when the pass folds, `i`
    /// applies and it has a case, an escape spelled again, else as written.
    fn literal(&mut self, unit: u32, span: Range<usize>) {
        if self.folds() && is_cased(unit) {
            self.out.push(u32::from('['));
            self.ranges(&closure(&[(unit, unit)]));
            self.out.push(u32::from(']'));
        } else if span.len() > 1 {
            class_unit(unit, &mut self.out);
        } else {
            self.out
                .extend_from_slice(self.units.get(span).unwrap_or_default());
        }
    }

    fn ranges(&mut self, ranges: &[(u32, u32)]) {
        for (low, high) in ranges {
            class_unit(*low, &mut self.out);
            if high > low {
                self.out.push(u32::from('-'));
                class_unit(*high, &mut self.out);
            }
        }
    }

    /// A class without `u`: its class escapes as written, its units and ranges spelled again,
    /// closed under `i` when `i` applies.
    fn legacy_class(&mut self) {
        self.at += 1;
        let negated = is(self.units.get(self.at), '^');
        if negated {
            self.at += 1;
        }
        let (mut sets, mut ranges) = (Vec::new(), Vec::new());
        while let Some(&unit) = self.units.get(self.at) {
            if unit == u32::from(']') {
                break;
            }
            let first = class_atom(self.units, self.at);
            self.at += atom_length(&first);
            let dash = is(self.units.get(self.at), '-') && !is(self.units.get(self.at + 1), ']');
            let second = dash.then(|| class_atom(self.units, self.at + 1));
            match (first, second) {
                (ClassAtom::Unit(low, _), Some(ClassAtom::Unit(high, length))) => {
                    ranges.push((low, high));
                    self.at += 1 + length;
                }
                (first, _) => self.class_member(first, &mut sets, &mut ranges),
            }
        }
        self.at += 1;
        let ranges = if self.folds() {
            closure(&ranges)
        } else {
            ranges
        };
        self.out.push(u32::from('['));
        if negated {
            self.out.push(u32::from('^'));
        }
        self.out.extend(sets);
        self.ranges(&ranges);
        self.out.push(u32::from(']'));
    }

    /// One member read alone: a class escape's text or a unit.
    fn class_member(&self, atom: ClassAtom, sets: &mut Vec<u32>, ranges: &mut Vec<(u32, u32)>) {
        match atom {
            ClassAtom::Unit(unit, _) => ranges.push((unit, unit)),
            ClassAtom::Set(length) => {
                let start = self.at - length;
                sets.extend_from_slice(self.units.get(start..self.at).unwrap_or_default());
            }
        }
    }

    /// The pattern without `u`, and whether a backreference that `i` applies to needs the text
    /// canonicalized; `None` when such a backreference meets a group that turns `i` on or off,
    /// which a text canonicalized whole cannot serve.
    pub(super) fn legacy(mut self) -> Option<(Vec<u32>, bool)> {
        let mut canonical_text = false;
        while let Some(&unit) = self.units.get(self.at) {
            match char::from_u32(unit).unwrap_or_default() {
                '\\' => match escape(self.units, self.at, self.groups, self.named) {
                    Escape::Unit(value, length) => {
                        self.literal(value, self.at..self.at + length);
                        self.at += length;
                    }
                    Escape::Verbatim(length) => self.copy(length),
                    Escape::Backreference(length) => {
                        canonical_text |= self.folds();
                        self.copy(length);
                    }
                    Escape::Backslash => {
                        self.out.extend([u32::from('\\'), u32::from('\\')]);
                        self.at += 1;
                    }
                },
                '[' => self.legacy_class(),
                '{' => match braced_length(self.units, self.at) {
                    Some(length) => self.copy(length),
                    None => {
                        self.out.extend([u32::from('\\'), u32::from('{')]);
                        self.at += 1;
                    }
                },
                '(' => self.open_group(self.fold == Folding::Pass),
                ')' => self.close_group(),
                _ => {
                    self.literal(unit, self.at..self.at + 1);
                    self.at += 1;
                }
            }
        }
        (!canonical_text || !self.modified).then_some((self.out, canonical_text))
    }

    /// The pattern with `u`, `\W` in a class that `i` applies to spelled as its ranges; `None`
    /// when there is none.
    pub(super) fn unicode_word_classes(mut self) -> Option<Vec<u32>> {
        let mut spelled = false;
        let mut in_class = false;
        while let Some(&unit) = self.units.get(self.at) {
            match (in_class, char::from_u32(unit).unwrap_or_default()) {
                (true, '\\') if self.folds() && is(self.units.get(self.at + 1), 'W') => {
                    self.out.extend(UNICODE_NOT_WORD.chars().map(u32::from));
                    self.at += 2;
                    spelled = true;
                }
                (_, '\\') => self.copy(2),
                (false, '[') => {
                    in_class = true;
                    self.copy(1);
                }
                (true, ']') => {
                    in_class = false;
                    self.copy(1);
                }
                (false, '(') => self.open_group(false),
                (false, ')') => self.close_group(),
                _ => self.copy(1),
            }
        }
        spelled.then_some(self.out)
    }
}

const fn atom_length(atom: &ClassAtom) -> usize {
    match atom {
        ClassAtom::Unit(_, length) | ClassAtom::Set(length) => *length,
    }
}
