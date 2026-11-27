import { coreModule } from '../wasm/bridge.ts';

/**
 * A tokenizer for the JavaScript of a workflow script (workflows.md, "What the saved script looks
 * like", "Edit a saved script"). The kit never runs a script and never parses it fully: it needs to
 * tell code from the text that only looks like code (comments, strings, template text, regular
 * expressions) and to read a literal. So this keeps words, numbers, strings and one-character
 * punctuation (`...` is one token), skips comments, and puts the tokens of a template's `${...}` in
 * the stream right after the template's own token.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/js_tokens.rs`, docs/RUST.md).
 */

const call = coreModule('format/js-tokens');

export type TokenForm = 'word' | 'number' | 'string' | 'regex' | 'punct';

export interface JsToken {
  readonly form: TokenForm;
  /** The source text of the token. */
  readonly text: string;
  readonly start: number;
  readonly end: number;
  /** A string's decoded text; `undefined` for a template with a `${...}` in it. */
  readonly value?: string;
}

export interface JsTokens {
  readonly tokens: readonly JsToken[];
  /** Why the text cannot be JavaScript at all: an unterminated string, comment, template or regex. */
  readonly problem: string | undefined;
}

/**
 * Whether a top-level scan can stop after the token just taken: `tokens` so far, `depth` the
 * brace depth after it. A scan that stops reports only the problems met so far.
 */
export type TokenStop = (tokens: readonly JsToken[], depth: number) => boolean;

/**
 * Tokens of `text`, up to where `stop` says, or all of them; never throws. A template's `${...}` is
 * scanned by a scanner of its own, one stack frame per level: nesting past the stack is a problem,
 * as an unterminated template is (judgment, spec/judgments.md).
 */
export const tokenize = (text: string, stop?: TokenStop): JsTokens => call('tokenize', text, stop);
