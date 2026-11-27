/**
 * Argument substitution (skills.md, "Available string substitutions" and "Pass arguments to
 * skills"): `$ARGUMENTS`, `$ARGUMENTS[N]`, `$N` and `$name` over the original text in one pass, so
 * an argument value is inserted as literal text and never expanded. `${CLAUDE_*}` is substituted
 * afterwards, over the result, as the docs say: "Claude Code still replaces `${CLAUDE_*}`
 * variables such as `${CLAUDE_SKILL_DIR}` after it inserts the arguments".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/substitute.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { SkillVariables } from './variables.ts';

const call = coreModule('skills/substitute');

export interface ArgumentInput {
  /** The text after the command name, as typed. */
  readonly rawArguments: string;
  /** `arguments` of the frontmatter: the names of `$name`, in argument order. */
  readonly names: readonly string[];
}

/**
 * `substituteArguments` over the pieces of one text, for a text cut at its `` !`command` ``
 * placeholders (`skills/expand`): a placeholder in any piece counts as receiving the arguments, and
 * the `ARGUMENTS:` line, when none did, ends the last piece. Each piece is scanned alone, so an
 * argument value inserted into one can never reach another.
 */
export const substituteArgumentPieces = (
  pieces: readonly string[],
  input: ArgumentInput,
): readonly string[] =>
  call('substituteArgumentPieces', pieces, {
    rawArguments: input.rawArguments,
    names: input.names,
  });

/**
 * `$ARGUMENTS`, `$ARGUMENTS[N]`, `$N` and `$name` in `content`. With arguments typed and no
 * placeholder receiving one, `ARGUMENTS: <value>` is appended to the end.
 */
export const substituteArguments = (content: string, input: ArgumentInput): string =>
  call('substituteArguments', content, { rawArguments: input.rawArguments, names: input.names });

export interface SubstitutionInput extends ArgumentInput {
  readonly vars: SkillVariables;
}

export interface Substitution {
  readonly text: string;
  readonly issues: readonly PluginIssue[];
}

/** Arguments first, then `${CLAUDE_*}` over the result (so over the inserted arguments too). */
export const substitute = (
  content: string,
  input: SubstitutionInput,
  path = '',
): Promise<Substitution> =>
  call.async(
    'substitute',
    content,
    { rawArguments: input.rawArguments, names: input.names, vars: input.vars },
    path,
  );
