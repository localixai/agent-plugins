/**
 * The words and redirections of one command (bash(1), "SHELL GRAMMAR" and "REDIRECTION"). An
 * unquoted `<` or `>` ends a word wherever it stands, so `echo x>/etc/x` is `echo x` writing
 * `/etc/x`; a descriptor number or `{name}` written right before the operator is part of it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/bash/words.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import { BASH_DIALECT, type ShellDialect } from './dialect.ts';

const call = coreModule('rules/bash/words');

/**
 * One shell word as the shell passes it to the command, and what in it the shell still expands
 * (the core's `rules/bash/unquote.rs`). Judgment (spec/judgments.md, oracle 2.1.319 and 2.1.321):
 * permissions.md does not say how a quoted command reads against a rule. Claude Code matches the
 * command name with its quoting removed, so `"rm" -rf x`, `r"m"`, `\rm` and `$'rm'` are all `rm`
 * to a rule, and leaves every argument as written: `echo "x y"` matches `Bash(echo "x y")` and not
 * `Bash(echo x y)`.
 */
interface WordValue {
  /** The word with its quotes, escapes and line continuations removed. */
  readonly value: string;
  /** A `$` or backtick outside single quotes: a variable or substitution may change the value. */
  readonly expands: boolean;
  /** A `*`, `?` or `[` outside quotes: the shell may expand the word to file names. */
  readonly globs: boolean;
}

/** A word or a redirection operator of a command, with where it starts and ends in the command. */
export interface Word {
  readonly text: string;
  readonly start: number;
  readonly end: number;
  /** A redirection operator (`>`, `2>&`, `<<`), whose target is the next word. */
  readonly operator: boolean;
}

/** A word of the command as written and as the shell reads it. */
export interface ShellWord extends Word, WordValue {}

/**
 * What a redirection does with its target: writes it, reads it, both (`<>`), names a descriptor
 * (`2>&1`, `<&-`), or holds text (a here-document or here-string).
 */
export type RedirectKind = 'output' | 'input' | 'both' | 'descriptor' | 'here';

export interface Redirect {
  /** The operator as written, its descriptor included (`2>`, `{fd}>`, `&>>`). */
  readonly operator: string;
  readonly kind: RedirectKind;
  /** The word after the operator; `undefined` when none follows, which the shell rejects. */
  readonly target: ShellWord | undefined;
}

/** One command: its words with every redirection taken out, and the redirections. */
export interface ShellCommand {
  readonly text: string;
  readonly words: readonly ShellWord[];
  readonly redirects: readonly Redirect[];
}

/** The words and operators of one command, quoted and nested parts kept whole. */
export const wordsOf = (command: string, dialect: ShellDialect = BASH_DIALECT): readonly Word[] =>
  call('wordsOf', command, dialect);

/** One command read into its words and redirections. */
export const readCommand = (text: string, dialect: ShellDialect = BASH_DIALECT): ShellCommand =>
  call('readCommand', text, dialect);

/**
 * A word as the shell passes it and as written. A Windows path is read both ways: its `\` is a
 * separator to the program though Bash reads it as an escape (`C:\Windows`, `\\server\share`).
 */
export const spellingsOf = (word: ShellWord): readonly string[] => call('spellingsOf', word);

/** The command's words as a rule reads them: the name unquoted, one space apart, continuations cut. */
export const spokenForm = (words: readonly ShellWord[], dialect: ShellDialect): string =>
  call('spokenForm', words, dialect);
