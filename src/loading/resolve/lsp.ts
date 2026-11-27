/**
 * An LSP server config resolved for this host.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/lsp.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../../format/issues.ts';
import type { LspServer } from '../../format/lsp-config.ts';
import type { SubstitutionScope } from './scope.ts';

const call = coreModule('loading/resolve/lsp');

/**
 * What Claude Code does when a field is unset (plugins/manifest-reference.md, "lspServers"):
 * `requestTimeout` 60000 milliseconds, `restartOnCrash` true, `diagnostics` true. The docs name no
 * default for `startupTimeout` or `maxRestarts`, and "when unset, no timeout applies" for
 * `shutdownTimeout`: those stay unset in the resolved server.
 */
export const LSP_DEFAULTS: {
  readonly requestTimeoutMs: 60000;
  readonly restartOnCrash: true;
  readonly diagnostics: true;
} = call('LSP_DEFAULTS');

/**
 * An LSP server config resolved for this host. The kit never starts a language server: the host's
 * LSP client reads this (command line, environment, timeouts, restart policy, the two payloads) and
 * runs it. The decisions the kit owns are in `lsp-runtime.ts`.
 */
export interface ResolvedLspServer {
  /** The key as the plugin wrote it. */
  readonly name: string;
  /**
   * The plugin it belongs to, by the id the host uses for it (`SubstitutionScope.pluginId`): what a
   * host keys a registration by. The same field on every resolved process (judgment, spec/judgments.md).
   */
  readonly pluginId: string;
  /** `command` with `${...}` substituted. */
  readonly command: string;
  readonly args: readonly string[];
  /** The server's own `env`, substituted; empty when it declares none. */
  readonly env: Readonly<Record<string, string>>;
  /** The variables the process receives besides `env`: the three path variables. */
  readonly environment: Readonly<Record<string, string>>;
  readonly workspaceFolder?: string;
  /** Always `stdio`: "Claude Code accepts `socket` but runs every server over stdio". */
  readonly transport: 'stdio';
  readonly extensionToLanguage: Readonly<Record<string, string>>;
  /** Not substituted: `${...}` does not resolve there (the table of fields lists four). */
  readonly initializationOptions?: unknown;
  readonly settings?: unknown;
  readonly startupTimeoutMs?: number;
  /** Undefined means no timeout: the host waits for a graceful shutdown. */
  readonly shutdownTimeoutMs?: number;
  readonly requestTimeoutMs: number;
  readonly restartOnCrash: boolean;
  /** Undefined means unbounded (judgment J-L2). */
  readonly maxRestarts?: number;
  readonly diagnostics: boolean;
}

export interface LspResolution {
  readonly resolved: ResolvedLspServer;
  readonly issues: readonly PluginIssue[];
}

/**
 * Resolves one server of a plugin (plugins/manifest-reference.md, "Where each variable resolves"):
 * `command`, `args`, `env` and `workspaceFolder` are substituted, the process gets the three path
 * variables (and the data folder is created for it), every other field is handed over as written
 * with the documented defaults applied.
 *
 * Judgment: `${user_config.KEY}` is read only in those four fields, although the page also says
 * "substituted in ... LSP server config": its table of fields where `${...}` resolves is the finer
 * statement, and `initializationOptions` and `settings` are data for the server, not the kit's text.
 */
export const resolveLspServer = (
  name: string,
  server: LspServer,
  scope: SubstitutionScope,
): Promise<LspResolution> => call.async('resolveLspServer', name, server, scope);
