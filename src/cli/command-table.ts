/**
 * The commands `claude` takes that this module knows (cli-reference.md, "CLI commands";
 * plugins/cli-reference.md, "claude plugin commands" and "claude plugin marketplace commands"): the
 * names, the aliases and who runs each. The kit has no CLI of its own; a `planned` command is one
 * whose outcome the kit models as a plan over ports, a `host-delegated` one is another product
 * feature (agent view, MCP servers and their OAuth) the host runs, recorded here so a host that routes
 * words to commands has one table. A registry typed over the documented names: no `switch`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/command_table.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('cli/command-table');

/** `claude plugin <subcommand>`; `claude plugin eval init` is a command of `eval`. */
export const PLUGIN_SUBCOMMANDS: readonly [
  'init',
  'install',
  'uninstall',
  'enable',
  'disable',
  'update',
  'list',
  'details',
  'configure',
  'prune',
  'eval',
  'tag',
  'test',
  'validate',
  'marketplace',
] = call('PLUGIN_SUBCOMMANDS');

export type PluginSubcommand = (typeof PLUGIN_SUBCOMMANDS)[number];

/** `claude plugin marketplace <subcommand>`. */
export const MARKETPLACE_SUBCOMMANDS: readonly ['add', 'list', 'remove', 'update'] =
  call('MARKETPLACE_SUBCOMMANDS');

export type MarketplaceSubcommand = (typeof MARKETPLACE_SUBCOMMANDS)[number];

/** `claude mcp login` and `claude mcp logout` are listed beside `claude mcp`. */
export const MCP_SUBCOMMANDS: readonly ['login', 'logout'] = call('MCP_SUBCOMMANDS');

export type McpSubcommand = (typeof MCP_SUBCOMMANDS)[number];

export type CommandName =
  | 'plugin'
  | `plugin ${PluginSubcommand}`
  | `plugin marketplace ${MarketplaceSubcommand}`
  | 'agents'
  | 'mcp'
  | `mcp ${McpSubcommand}`;

export const COMMAND_HANDLINGS: readonly ['planned', 'host-delegated'] = call('COMMAND_HANDLINGS');

export type CommandHandling = (typeof COMMAND_HANDLINGS)[number];

export interface CommandRow {
  /** Other words that run the same command, at the same position. */
  readonly aliases: readonly string[];
  readonly handling: CommandHandling;
  /**
   * What a word that is none of the command's subcommands is: `unknown` for `claude plugin` and
   * `claude plugin marketplace` (`claude plugin add` is "unknown command 'add'"), `argument` for a
   * command that takes arguments of its own.
   */
  readonly unlistedWord: 'unknown' | 'argument';
}

/** Every command with its aliases; a name added to a union above fails the typecheck until it has a row. */
export const COMMAND_TABLE: { readonly [C in CommandName]: CommandRow } = call('COMMAND_TABLE');

export interface ResolvedCommand {
  readonly command: CommandName;
  readonly handling: CommandHandling;
  /** The words after the command, for its own parsing. */
  readonly rest: readonly string[];
}

/**
 * The command the words name, by name or alias, at each level. `claude plugin add` is unknown; a
 * word below `claude mcp` or `claude agents` that the table does not list is that command's own
 * argument, so the host keeps it in `rest`.
 */
export const resolveCommand = (words: readonly string[]): ResolvedCommand | undefined =>
  call('resolveCommand', words);
