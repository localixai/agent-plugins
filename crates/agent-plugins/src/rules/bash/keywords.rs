//! Port of `src/rules/bash/keywords.ts` at sha256:313e6ef8b3a1.
//!
//! Not switched: `stripKeywords` pushes each name it reads into the caller's own `defined` array,
//! which a call across the boundary cannot reach (it gets a copy). The scanner that calls it
//! (`scanner.ts`) uses this port once it is ported itself; until then the package runs the
//! TypeScript and the vectors hold this to its answers.

use std::sync::LazyLock;

use regex::Regex;

use crate::js::{Pattern, SPACE_CLASS, regex_test, trim};

/// Shell keywords that stand where a command would: they start a body (`do`, `then`, `{`) or a
/// condition (`if`, `while`, `until`, `!`), and a rule sees the command after them. permissions.md
/// ("Compound commands") names "a control-flow body such as a `for` loop"; the keyword list is the
/// kit's own (spec/judgments.md). `case x in` and a pattern label such as `a)` or `*)` stand before
/// the command of a `case` body in the same way.
///
/// `/^(?:do|then|else|if|elif|while|until|!|\{|case\s.*?\sin|\(?[^\s()]+\))(?:\s+|$)/s`.
static LEADING: Pattern = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?s)^(?:do|then|else|if|elif|while|until|!|\{{|case[{SPACE_CLASS}].*?[{SPACE_CLASS}]in|\(?[^{SPACE_CLASS}()]+\))(?:[{SPACE_CLASS}]+|$)"
    ))
    .ok()
});

/// A function definition's head, `name()` or `function name` (bash(1), "Shell Function
/// Definitions"): the command after it is the function's body, which a rule sees like any other.
///
/// `/^(?:function\s+([^\s()]+)(?:\s*\(\s*\))?|([^\s()=$'"`]+)\s*\(\s*\))(?:\s+|$)/s`.
static FUNCTION_HEAD: Pattern = LazyLock::new(|| {
    Regex::new(&format!(
        r#"(?s)^(?:function[{SPACE_CLASS}]+([^{SPACE_CLASS}()]+)(?:[{SPACE_CLASS}]*\([{SPACE_CLASS}]*\))?|([^{SPACE_CLASS}()=$'"`]+)[{SPACE_CLASS}]*\([{SPACE_CLASS}]*\))(?:[{SPACE_CLASS}]+|$)"#
    ))
    .ok()
});

/// Keywords that end a block, and the headers that only name what a loop or case ranges over.
///
/// `/^(?:done|fi|esac|\}|(?:for|select)(?:\s.*)?)$/s`.
static STRUCTURE: Pattern = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?s)^(?:done|fi|esac|\}}|(?:for|select)(?:[{SPACE_CLASS}].*)?)$"
    ))
    .ok()
});

/// The command a segment holds once its leading keywords and function heads are removed, or
/// `undefined` when the segment is only structure (`done`, `fi`, `for f in a b`, an empty body). The
/// name of each function the segment defines is pushed to `defined`.
pub fn strip_keywords(segment: &str, defined: &mut Vec<String>) -> Option<String> {
    let mut text = segment;
    loop {
        let head = FUNCTION_HEAD
            .as_ref()
            .and_then(|pattern| pattern.captures(text));
        if let Some(found) = &head {
            let name = found.get(1).or_else(|| found.get(2));
            defined.push(name.map_or("", |name| name.as_str()).to_owned());
        }
        let length = head
            .and_then(|found| found.get(0))
            .or_else(|| LEADING.as_ref().and_then(|pattern| pattern.find(text)))
            .map(|found| found.end());
        let Some(length) = length else {
            break;
        };
        text = text.get(length..).unwrap_or_default();
    }
    let command = trim(text);
    if command.is_empty() || regex_test(&STRUCTURE, command) {
        None
    } else {
        Some(command.to_owned())
    }
}

crate::registry::exports! {
    "rules/bash/keywords" {
        "stripKeywords" => ok |segment: String, defined: Vec<String>| {
            let mut defined = defined;
            strip_keywords(&segment, &mut defined)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_patterns_compile() {
        assert!(LEADING.is_some());
        assert!(FUNCTION_HEAD.is_some());
        assert!(STRUCTURE.is_some());
    }

    // bun -e 'import { stripKeywords } from "./src/rules/bash/keywords.ts"; const d = [];
    //   console.log(JSON.stringify([stripKeywords("do ls", d), stripKeywords("then\u0085ls", d),
    //   stripKeywords("f﻿() { ls; }", d), stripKeywords("for x", d), d]))'
    // JavaScript's `\s` holds U+00A0, U+2028 and U+FEFF and not U+0085.
    #[test]
    fn keywords_end_at_javascript_whitespace() {
        let mut defined = Vec::new();
        assert_eq!(
            strip_keywords("do\u{a0}ls", &mut defined).as_deref(),
            Some("ls")
        );
        assert_eq!(
            strip_keywords("then\u{85}ls", &mut defined).as_deref(),
            Some("then\u{85}ls")
        );
        assert_eq!(
            strip_keywords("f\u{feff}() { ls; }", &mut defined).as_deref(),
            Some("ls; }")
        );
        assert_eq!(strip_keywords("for\u{2028}x", &mut defined), None);
        assert_eq!(defined, ["f"]);
    }
}
