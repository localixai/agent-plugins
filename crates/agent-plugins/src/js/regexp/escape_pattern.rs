//! `EscapeRegExpPattern` (ECMA-262, `RegExp.prototype.source`) as JavaScriptCore spells it: a `/`
//! outside a class and not escaped is written `\/`, a line terminator as its escape (`\n`, `\r`,
//! ` `, ` `; an escaped one keeps its backslash), and the empty pattern as `(?:)`.

/// `source` as `RegExp.prototype.source` prints it.
pub(super) fn escape_pattern(source: &str) -> String {
    if source.is_empty() {
        return "(?:)".to_owned();
    }
    let mut out = String::with_capacity(source.len());
    let (mut escaped, mut in_class) = (false, false);
    for character in source.chars() {
        let terminator = match character {
            '\n' => Some("n"),
            '\r' => Some("r"),
            '\u{2028}' => Some("u2028"),
            '\u{2029}' => Some("u2029"),
            _ => None,
        };
        if let Some(letters) = terminator {
            if !escaped {
                out.push('\\');
            }
            out.push_str(letters);
            escaped = false;
            continue;
        }
        if character == '/' && !escaped && !in_class {
            out.push('\\');
        }
        out.push(character);
        match (escaped, character) {
            (true, _) => escaped = false,
            (false, '\\') => escaped = true,
            (false, '[') => in_class = true,
            (false, ']') => in_class = false,
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::escape_pattern;

    #[test]
    fn escapes_slashes_and_line_breaks() {
        assert_eq!(escape_pattern(""), "(?:)");
        assert_eq!(escape_pattern("[/]/\\/"), "[/]\\/\\/");
        assert_eq!(escape_pattern("a\nb\\\n"), "a\\nb\\n");
        assert_eq!(escape_pattern("[\\]/]/"), "[\\]/]\\/");
    }
}
