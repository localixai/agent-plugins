import type { BashInput, MonitorInput } from '../../protocol/official-tools.ts';
import type { Assert, Equal } from '../../protocol/type-equality.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * What differs between the command tools, the one matcher reading it: how a command splits, the
 * forms of a subcommand a rule is tried against, and how a pattern is compared with one. The
 * matcher and the shells are the core's; only the command field is read here too.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/bash/shell.rs`, docs/RUST.md).
 */
const call = coreModule('rules/bash/shell');

/** The field that holds the command: the SDK's `BashInput` and `MonitorInput` both have it. */
const COMMAND_FIELD = 'command' as const satisfies keyof BashInput & keyof MonitorInput;

/** @public A compile-time drift guard: `command` is a string in `BashInput`. */
export type BashCommandIsText = Assert<Equal<BashInput[typeof COMMAND_FIELD], string>>;

/** @public A compile-time drift guard: `command` is an optional string in `MonitorInput`. */
export type MonitorCommandIsText = Assert<
  Equal<NonNullable<MonitorInput[typeof COMMAND_FIELD]>, string>
>;

/**
 * The command text a call carries: `command` for Bash, Monitor and PowerShell (hooks.md). The input
 * is `unknown` because a hook or a model sends whatever it likes; the field name is the SDK's.
 */
export const commandOf = (input: unknown): string | undefined => call('commandOf', input);
