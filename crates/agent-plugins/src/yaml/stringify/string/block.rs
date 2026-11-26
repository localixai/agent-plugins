//! A string as a block scalar (`blockString` of `dist/stringify/stringifyString.js`): `|` or `>`
//! with its indentation and chomping indicators, its lines indented and, when folded, broken.

use super::{after_newlines, quoted_string, root_indent};
use crate::js::code_unit_length;
use crate::yaml::nodes::{Scalar, ScalarType};
use crate::yaml::stringify::fold::{Mode, fold_flow_lines, line_length_over_limit};
use crate::yaml::stringify::{Context, Out, stringify_comment};

/// `end.replace(blockEndNewlines, '$&' + indent)`: the indent after each run of line breaks
/// that is not at the end.
fn indent_inner_newline_runs(end: &str, indent: &str) -> String {
    let trimmed = end.trim_end_matches('\n');
    let tail = end.get(trimmed.len()..).unwrap_or_default();
    format!("{}{tail}", after_newlines(trimmed, indent))
}

/// `/(?:^|\n)([\t ].*)(?:([\n\t ]*)\n(?![\n\t ]))?/g` replaced by `$1$2`: a more-indented line
/// keeps its line break before it, and the blanks and breaks after it lose their last break.
fn keep_more_indented(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < chars.len() {
        let at_start = index == 0 && matches!(chars.first(), Some('\t' | ' '));
        let after_break =
            chars.get(index) == Some(&'\n') && matches!(chars.get(index + 1), Some('\t' | ' '));
        if !at_start && !after_break {
            out.extend(chars.get(index));
            index += 1;
            continue;
        }
        let line_start = if at_start { index } else { index + 1 };
        let line_end = (line_start..chars.len())
            .find(|at| matches!(chars.get(*at), Some('\n' | '\r' | '\u{2028}' | '\u{2029}')))
            .unwrap_or(chars.len());
        out.extend(chars.get(line_start..line_end).unwrap_or_default());
        let run_end = (line_end..chars.len())
            .find(|at| !matches!(chars.get(*at), Some('\n' | '\t' | ' ')))
            .unwrap_or(chars.len());
        if run_end > line_end && chars.get(run_end - 1) == Some(&'\n') {
            out.extend(chars.get(line_end..run_end - 1).unwrap_or_default());
            index = run_end;
        } else {
            index = line_end;
        }
    }
    out
}

/// `blockString`.
pub(super) fn block_string(
    scalar: &Scalar,
    value: &str,
    comment: Option<&str>,
    ctx: &Context<'_>,
    out: &mut Out,
) -> String {
    // 1. A block can't end in whitespace unless the last line is non-empty.
    // 2. Strings consisting of only whitespace are best rendered explicitly.
    let tail = value.trim_end_matches(['\t', ' ']);
    if tail.len() < value.len() && tail.ends_with('\n') {
        return quoted_string(value, ctx);
    }
    let indent = root_indent(ctx, value, ctx.force_block_indent);
    let literal = match scalar.kind {
        Some(ScalarType::BlockFolded) => false,
        Some(ScalarType::BlockLiteral) => true,
        _ => !line_length_over_limit(value, ctx.line_width, code_unit_length(&indent)),
    };
    if value.is_empty() {
        return if literal { "|\n" } else { ">\n" }.to_owned();
    }
    // Determine chomping from whitespace at the value's end.
    let content = value.trim_end_matches(['\n', '\t', ' ']);
    let mut end = value.get(content.len()..).unwrap_or_default().to_owned();
    let chomp = match end.find('\n') {
        None => "-",
        Some(at) if value == end || at != end.len() - 1 => {
            out.chomp_keep = true;
            "+"
        }
        Some(_) => "",
    };
    let mut value = content.to_owned();
    if !end.is_empty() {
        if end.ends_with('\n') {
            end.pop();
        }
        end = indent_inner_newline_runs(&end, &indent);
    }
    // Determine the indent indicator from whitespace at the value's start.
    let mut start_with_space = false;
    let mut start_nl_pos = None;
    for (at, character) in value.char_indices() {
        match character {
            ' ' => start_with_space = true,
            '\n' => start_nl_pos = Some(at),
            _ => break,
        }
    }
    // Up to the last line break of the leading blanks.
    let start_len = start_nl_pos.map_or(0, |at| at + 1);
    let mut start = value.get(..start_len).unwrap_or_default().to_owned();
    if !start.is_empty() {
        value = value.get(start_len..).unwrap_or_default().to_owned();
        start = after_newlines(&start, &indent);
    }
    // The root is at -1.
    let indent_size = if indent.is_empty() { "1" } else { "2" };
    // A leading `|` or `>` is added later.
    let mut header = format!("{}{chomp}", if start_with_space { indent_size } else { "" });
    if let Some(comment) = comment.filter(|comment| !comment.is_empty()) {
        let flat: String = collapse_breaks(comment);
        header.push(' ');
        header.push_str(&stringify_comment(&flat));
        out.commented = true;
    }
    if !literal {
        let folded = after_newlines(&keep_more_indented(&double_breaks(&value)), &indent);
        let text = format!("{start}{folded}{end}");
        let refuse_overflow = scalar.kind != Some(ScalarType::BlockFolded);
        let (body, overflow) = fold_flow_lines(
            &text,
            &indent,
            Mode::Block,
            Some(code_unit_length(&ctx.indent)),
            ctx.line_width,
        );
        if !(refuse_overflow && overflow) {
            return format!(">{header}\n{indent}{body}");
        }
    }
    let value = after_newlines(&value, &indent);
    format!("|{header}\n{indent}{start}{value}{end}")
}

/// `value.replace(/\n+/g, '\n$&')`: one more line break in each run.
fn double_breaks(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut previous = None;
    for character in text.chars() {
        if character == '\n' && previous != Some('\n') {
            out.push('\n');
        }
        out.push(character);
        previous = Some(character);
    }
    out
}

/// `comment.replace(/ ?[\r\n]+/g, ' ')`.
fn collapse_breaks(comment: &str) -> String {
    let mut out = String::with_capacity(comment.len());
    let mut chars = comment.chars().peekable();
    while let Some(character) = chars.next() {
        let breaks_next = character == ' ' && matches!(chars.peek(), Some('\r' | '\n'));
        if breaks_next || matches!(character, '\r' | '\n') {
            while matches!(chars.peek(), Some('\r' | '\n')) {
                chars.next();
            }
            out.push(' ');
        } else {
            out.push(character);
        }
    }
    out
}
