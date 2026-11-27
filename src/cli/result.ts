/**
 * What a `claude plugin` command reports (plugins/cli-reference.md, "claude plugin commands",
 * "JSON result format", "JSON result for marketplace commands"): the human line, the exit code and,
 * with `--json`, one object on the last line of stdout. The kit has no CLI of its own, so a command
 * is a plan whose outcome is one of these values; a host prints `renderCommand`'s lines and exits with its
 * code. The streams a human line goes to, and the exit code of a refusal that prints no result line,
 * are in spec/judgments.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/result.rs`, docs/RUST.md).
 */
import type { JsonValue } from '../foundation/plain-data.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('cli/result');

export const COMMAND_OUTCOMES: readonly ['ok', 'failed'] = call('COMMAND_OUTCOMES');
export type CommandOutcome = (typeof COMMAND_OUTCOMES)[number];

/** `0` success, `1` failure, `2` an unexpected error (`validate` only). */
export type ExitCode = 0 | 1 | 2;

/** Fields beyond the three that are always present: `marketplace`, `pluginId`, `scope`… */
export type ResultFields = Readonly<Record<string, JsonValue>>;

export interface PluginCommandResult {
  /** The subcommand that ran, such as `install` or `marketplace-add`. */
  readonly command: string;
  readonly outcome: CommandOutcome;
  /** The human-readable description of the result, the line printed without `--json`. */
  readonly message: string;
  /** Why a command failed, such as `invalid_source`; present only when it applies. */
  readonly failureCode?: string;
  readonly fields?: ResultFields;
}

/**
 * What a command ends with. Only `result` can print the JSON line: a usage error ("an invalid
 * `--scope`"), a name that "can print no result line" and an unexpected error give a reason on
 * stderr alone.
 */
export type CommandOutput =
  | { readonly kind: 'result'; readonly result: PluginCommandResult }
  | { readonly kind: 'usage-error'; readonly reason: string }
  | { readonly kind: 'no-result-line'; readonly reason: string }
  | { readonly kind: 'unexpected-error'; readonly reason: string };

export interface Rendered {
  readonly stdout: readonly string[];
  readonly stderr: readonly string[];
  readonly exitCode: ExitCode;
}

export interface RenderOptions {
  /** `--json`: the last line of stdout is the result object instead of the human line. */
  readonly json: boolean;
  /** What the command printed before its result (a marketplace-declared command, for one). */
  readonly before?: readonly string[];
}

interface Extras {
  readonly failureCode?: string;
  readonly fields?: ResultFields;
}

export const commandSucceeded = (
  command: string,
  message: string,
  extras: Extras = {},
): CommandOutput => call('commandSucceeded', command, message, extras);

export const commandFailed = (
  command: string,
  message: string,
  extras: Extras = {},
): CommandOutput => call('commandFailed', command, message, extras);

export type UsageError = Extract<CommandOutput, { kind: 'usage-error' }>;
export const usageError = (reason: string): UsageError => call('usageError', reason);
export const noResultLine = (reason: string): CommandOutput => call('noResultLine', reason);
export const unexpectedError = (reason: string): CommandOutput => call('unexpectedError', reason);

/** The `command` values of the marketplace commands ("`marketplace-add`, `marketplace-remove`, or `marketplace-update`"). */
export const MARKETPLACE_COMMANDS: readonly [
  'marketplace-add',
  'marketplace-remove',
  'marketplace-update',
] = call('MARKETPLACE_COMMANDS');
export type MarketplaceCommand = (typeof MARKETPLACE_COMMANDS)[number];

export const marketplaceSuccess = (
  command: MarketplaceCommand,
  marketplace: string | undefined,
  message: string,
): CommandOutput => call('marketplaceSuccess', command, marketplace, message);

export const marketplaceFailure = (
  command: MarketplaceCommand,
  marketplace: string | undefined,
  message: string,
  failureCode: string,
): CommandOutput => call('marketplaceFailure', command, marketplace, message, failureCode);

/** What a host prints and exits with for `output`. */
export const renderCommand = (output: CommandOutput, options: RenderOptions): Rendered =>
  call('renderCommand', output, options);
