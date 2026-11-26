//! Comments as text (`dist/stringify/stringifyComment.js`): `#` before each line, indented, or
//! after a line.

/// `stringifyComment`: each line that is not empty prefixed with `#`, a line of one space
/// replaced by it.
pub(crate) fn stringify_comment(text: &str) -> String {
    map_lines(text, |line| {
        if line.is_empty() {
            String::new()
        } else if line == " " {
            "#".to_owned()
        } else {
            format!("#{line}")
        }
    })
}

/// Each line of a text (split as a multiline JavaScript pattern splits it: at `\n`, `\r`,
/// U+2028 and U+2029) rewritten, the breaks kept.
fn map_lines(text: &str, mut rewrite: impl FnMut(&str) -> String) -> String {
    let mut out = String::with_capacity(text.len());
    let mut line = String::new();
    for character in text.chars() {
        if matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
            out.push_str(&rewrite(&line));
            out.push(character);
            line.clear();
        } else {
            line.push(character);
        }
    }
    out.push_str(&rewrite(&line));
    out
}

/// `indentComment`.
pub(crate) fn indent_comment(comment: &str, indent: &str) -> String {
    if !comment.is_empty() && comment.chars().all(|character| character == '\n') {
        return comment.get(1..).unwrap_or_default().to_owned();
    }
    if indent.is_empty() {
        return comment.to_owned();
    }
    map_lines(comment, |line| {
        if line.chars().all(|character| character == ' ') {
            line.to_owned()
        } else {
            format!("{indent}{line}")
        }
    })
}

/// `lineComment`.
pub(crate) fn line_comment(text: &str, indent: &str, comment: &str) -> String {
    if text.ends_with('\n') {
        indent_comment(comment, indent)
    } else if comment.contains('\n') {
        format!("\n{}", indent_comment(comment, indent))
    } else {
        format!("{}{comment}", if text.ends_with(' ') { "" } else { " " })
    }
}
