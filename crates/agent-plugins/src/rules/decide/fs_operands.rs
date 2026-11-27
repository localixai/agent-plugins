//! Ported from `src/rules/decide/fs-operands.ts`; only the core uses it now, so it has no wrapper.
//!
//! The filesystem commands `acceptEdits` approves (permission-modes.md, "Auto-approve file edits with
//! acceptEdits mode": "`mkdir`, `touch`, `rm`, `rmdir`, `mv`, `cp`, and `sed`") and the operands each
//! writes, or `undefined` when the call does more than write them.

use super::sed_script::sed_operands;
use crate::js::code_unit_length;
use crate::rules::bash::words::ShellWord;

fn is_option(word: &ShellWord) -> bool {
    word.value.starts_with('-') && word.value != "-"
}

/// Every word that is not an option; after `--`, every word.
#[expect(
    clippy::unnecessary_wraps,
    reason = "one signature for every entry of OPERANDS, where touch and sed may refuse"
)]
fn plain_operands(args: &[ShellWord]) -> Option<Vec<ShellWord>> {
    let end = args.iter().position(|word| word.value == "--");
    let before = &args[..end.unwrap_or(args.len())];
    let after = end.map_or(&[][..], |end| args.get(end + 1..).unwrap_or_default());
    Some(
        before
            .iter()
            .filter(|word| !is_option(word))
            .chain(after)
            .cloned()
            .collect(),
    )
}

const TARGET_DIRECTORY: &str = "--target-directory";

/// `SHORT_TARGET`, `/^-[A-Za-z]*t(.*)$/s`: `-t DIR` or a cluster ending in it (`-vt DIR`), and one
/// with the directory attached (`-vt/etc`): the text after the last `t` of the letters.
fn short_target(option: &str) -> Option<&str> {
    let letters = option.strip_prefix('-')?;
    let run = letters
        .find(|character: char| !character.is_ascii_alphabetic())
        .unwrap_or(letters.len());
    let last_t = letters.get(..run)?.rfind('t')?;
    letters.get(last_t + 1..)
}

/// A long option GNU reads as `--target-directory`: the name or a prefix of it, three letters on.
fn is_target_directory(name: &str) -> bool {
    code_unit_length(name) >= 3 && TARGET_DIRECTORY.starts_with(name)
}

/// The directory written into a target option (`--target-directory=DIR`, `-vtDIR`), `''` for one
/// whose directory is the next word, `undefined` for another option.
fn attached_target(option: &str) -> Option<String> {
    if option.starts_with("--") {
        let mut pieces = option.split('=');
        let name = pieces.next().unwrap_or("");
        let value: Vec<&str> = pieces.collect();
        return is_target_directory(name).then(|| value.join("="));
    }
    short_target(option).map(str::to_owned)
}

/// The directory one option of `mv` or `cp` names, and how many words it took.
fn target_of(args: &[ShellWord], at: usize) -> (Option<ShellWord>, usize) {
    let word = args.get(at);
    let attached = word.and_then(|word| attached_target(&word.value));
    let (Some(word), Some(attached)) = (word, attached) else {
        return (None, 1);
    };
    if attached.is_empty() {
        (args.get(at + 1).cloned(), 2)
    } else {
        (
            Some(ShellWord {
                value: attached,
                ..word.clone()
            }),
            1,
        )
    }
}

/// `mv` and `cp`: the operands and the target directory an option names (`-t`, `--target-directory`).
#[expect(
    clippy::unnecessary_wraps,
    reason = "one signature for every entry of OPERANDS, where touch and sed may refuse"
)]
fn move_operands(args: &[ShellWord]) -> Option<Vec<ShellWord>> {
    let mut operands = Vec::new();
    let mut at = 0;
    while at < args.len() {
        let Some(word) = args.get(at).filter(|word| word.value != "--") else {
            operands.extend(args.get(at + 1..).unwrap_or_default().iter().cloned());
            break;
        };
        if !is_option(word) {
            operands.push(word.clone());
            at += 1;
            continue;
        }
        let (target, taken) = target_of(args, at);
        operands.extend(target);
        at += taken;
    }
    Some(operands)
}

/// `touch` takes no option here: `-r` and `-d` read another file or a date the kit does not judge.
fn touch_operands(args: &[ShellWord]) -> Option<Vec<ShellWord>> {
    if args.iter().any(is_option) {
        None
    } else {
        Some(args.to_vec())
    }
}

type Operands = fn(&[ShellWord]) -> Option<Vec<ShellWord>>;

const OPERANDS: [(&str, Operands); 7] = [
    ("mkdir", plain_operands),
    ("touch", touch_operands),
    ("rm", plain_operands),
    ("rmdir", plain_operands),
    ("mv", move_operands),
    ("cp", move_operands),
    ("sed", sed_operands),
];

/// Whether `acceptEdits` knows the program.
#[must_use]
pub fn is_filesystem_program(program: &str) -> bool {
    OPERANDS.iter().any(|(name, _)| *name == program)
}

/// The operands a filesystem command writes (its words after the program), or `None`.
#[must_use]
pub fn filesystem_operands(program: &str, args: &[ShellWord]) -> Option<Vec<ShellWord>> {
    OPERANDS
        .iter()
        .find(|(name, _)| *name == program)
        .and_then(|(_, operands)| operands(args))
}
