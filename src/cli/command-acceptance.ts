import type { Sha256 } from '../ports/marketplace.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { UsageError } from './result.ts';

const call = coreModule('cli/command-acceptance');

/**
 * Accepting a marketplace-declared command before `claude plugin install` or `update` runs it
 * (plugins/cli-reference.md, "plugin install": `-y, --yes`, `--accept-command <sha256>`, the lines
 * `Aborted.` and `Aborted — the command was not run.`; "Accept a displayed install command";
 * plugins/host-marketplace.md, "How users accept a headersHelper command"). "Claude Code first prints
 * the command and asks `Run this command now? [y/N]`"; `-y` accepts it "Ignored when the command runs
 * inside a Claude Code session"; `--accept-command` "Can't be combined with `-y`" and "has no effect
 * inside a Claude Code session"; "The `sha256` counts as acceptance for exactly that command, plugin,
 * and marketplace catalog. If any of them changed since the command was displayed, Claude Code
 * doesn't accept the `sha256` and shows the command again. A change that the run's own marketplace
 * refresh fetches also counts as such a change." Judgments (spec/judgments.md): what the digest
 * covers (`digestOf`), that a `--accept-command` that does not match never prompts, and the line a
 * command source gets when it cannot be confirmed without a terminal.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/command_acceptance.rs`, docs/RUST.md).
 */

/** What the entry asks Claude Code to run. A `headersHelper` wins when an entry has both. */
export type CommandOrigin = 'command-source' | 'headers-helper';

export interface DisplayedCommand {
  readonly command: string;
  /** The plugin id the command belongs to. */
  readonly plugin: string;
  readonly origin: CommandOrigin;
}

export interface AcceptanceFlags {
  readonly yes?: boolean;
  readonly acceptCommand?: string;
}

/** "Can't be combined with `-y`": a flag problem, nothing is printed but the reason. */
export const exclusiveAcceptFlags = (flags: AcceptanceFlags): UsageError | undefined =>
  call('exclusiveAcceptFlags', flags);

/** The object a `--json` run's `failed` result carries when it displays a command and does not run it. */
export interface ShownCommand {
  readonly command: string;
  readonly plugin: string;
  readonly sha256: string;
  /** Present when `--accept-command` was passed: whether its value is the digest of the command now displayed. */
  readonly acceptCommandMatched?: boolean;
}

export interface AcceptanceRequest {
  readonly shown: DisplayedCommand;
  /** The marketplace catalog the run read, after its own refresh: a change the refresh fetched changes the digest. */
  readonly catalog: string;
  readonly flags: AcceptanceFlags;
  /** The command runs inside a Claude Code session (the Bash tool, a hook). */
  readonly session: boolean;
  /** Both stdin and stdout are a terminal. */
  readonly tty: boolean;
  /** The answer to `Run this command now? [y/N]`, once the host has asked. */
  readonly answer?: boolean;
}

export const PROMPT: string = call('PROMPT');

export type CommandAcceptance =
  | { readonly run: true }
  /** The host prints the command, asks `prompt` and calls again with the answer. */
  | {
      readonly run: false;
      readonly ask: true;
      readonly prompt: string;
      readonly shownCommand: ShownCommand;
    }
  | {
      readonly run: false;
      readonly ask: false;
      readonly line: string;
      readonly shownCommand: ShownCommand;
    };

/** The digest `--accept-command` names: SHA-256 of the command, the plugin and the catalog it came from (judgment: the docs do not say how it is built). */
export const digestOf = (
  shown: DisplayedCommand,
  catalog: string,
  sha256: Sha256,
): Promise<string> => call.async('digestOf', shown, catalog, sha256);

/**
 * Whether the displayed command runs. Inside a session neither `-y` nor `--accept-command` counts.
 * A digest that does not match the command now displayed is shown again with
 * `acceptCommandMatched: false` and never prompts.
 */
export const decideAcceptance = (
  request: AcceptanceRequest,
  sha256: Sha256,
): Promise<CommandAcceptance> => call.async('decideAcceptance', request, sha256);
