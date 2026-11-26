/**
 * The `<claude-code-hint />` tag a CLI writes to stderr to recommend its plugin (plugins/cli-hints.md,
 * "Emit the hint", "Hint format", "Check when the prompt appears", "Preview what the user sees"): pure,
 * over output text and facts the host holds. The kit strips the lines and decides whether a prompt
 * shows; the host draws the dialog and installs at user scope.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/plugin_hints.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/plugin-hints');

/** A hint that parsed: `value` is `name@marketplace`. */
export interface PluginHint {
  readonly v: string;
  readonly type: string;
  readonly value: string;
}

export interface StrippedOutput {
  /** The output the model sees. */
  readonly output: string;
  /** Every hint line removed, in order; a line whose attributes are incomplete is removed but not listed. */
  readonly hints: readonly PluginHint[];
}

/**
 * "The tag must occupy its own line; Claude Code ignores a tag embedded mid-line." A tag on its own line
 * is removed from the output "even when `v` or `type` is unrecognized".
 */
export const stripHints = (output: string): StrippedOutput => call('stripHints', output);

/** `v` `1` and `type` `plugin` are "the only supported value"; `value` is `name@marketplace`. */
export const supportedHint = (
  hint: PluginHint,
): { readonly plugin: string; readonly marketplace: string } | undefined =>
  call('supportedHint', hint);

/** "Emit the tag only when `CLAUDECODE` or `CLAUDE_CODE_CHILD_SESSION` is set": what a CLI's gate reads. */
export const shouldEmitHint = (env: {
  readonly get: (name: string) => string | undefined;
}): boolean => call('shouldEmitHint', env);

/** The one-line tag for a plugin in an official marketplace. */
export const hintLine = (value: string): string => call('hintLine', value);

/** "One prompt ever per plugin ... and none once 100 plugins have been prompted for on that machine." */
export const HINT_PROMPT_LIMIT: number = call('HINT_PROMPT_LIMIT');

/** What the host knows when a hint arrives. */
export interface PromptFacts {
  /** An interactive terminal session; `claude -p`, a subagent run and hook command output are not. */
  readonly interactiveTerminal: boolean;
  /** The workspace is local and the session is attended (not `--cloud`, Remote Control or a teammate). */
  readonly localAttended: boolean;
  /** The plugin is in the local copy of an official marketplace, not installed, and no policy blocks it. */
  readonly installable: boolean;
  /** Claude Code's analytics are on. */
  readonly analyticsOn: boolean;
  /** A prompt already showed in this session. */
  readonly promptedThisSession: boolean;
  /** The plugins already prompted for on this machine. */
  readonly promptedPlugins: readonly string[];
  /** The user chose "No, and don't show plugin installation hints again". */
  readonly hintsTurnedOff: boolean;
}

export type PromptVerdict =
  | { readonly show: true; readonly plugin: string; readonly marketplace: string }
  | { readonly show: false; readonly reason: PromptRefusal };

export type PromptRefusal =
  | 'unsupported'
  | 'not-official'
  | 'not-interactive'
  | 'not-installable'
  | 'analytics-off'
  | 'already-prompted-this-session'
  | 'already-prompted-for-plugin'
  | 'prompt-limit'
  | 'hints-turned-off'
  | 'not-local-attended';

/** Whether the **Plugin recommendation** dialog shows for a hint. */
export const promptVerdict = (hint: PluginHint, facts: PromptFacts): PromptVerdict =>
  call('promptVerdict', hint, facts);

/** The three answers of the dialog. */
export type HintAnswer = 'install' | 'no' | 'never' | 'timeout';

export interface HintEffect {
  /** Install the plugin at user scope. */
  readonly install: boolean;
  /** Turn off future hint prompts for the user. */
  readonly turnOff: boolean;
}

/** "No answer for 30 seconds: counts as No." */
export const HINT_ANSWER_TIMEOUT_MS: number = call('HINT_ANSWER_TIMEOUT_MS');

export const answerEffect = (answer: HintAnswer): HintEffect => call('answerEffect', answer);

/** The dialog's text: the command's first word, the plugin and the marketplace. */
export const dialogLines = (
  command: string,
  plugin: string,
  marketplace: string,
  description?: string,
): readonly string[] => call('dialogLines', command, plugin, marketplace, description);
