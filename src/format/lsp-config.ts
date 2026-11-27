import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';
import type { LSP_TRANSPORTS } from './lsp-schema.ts';

/**
 * LSP server entries of a plugin (plugins/manifest-reference.md, "lspServers"; plugins/components.md,
 * "LSP servers"). The SDK has no LSP config type (no match in sdk.d.ts), so `LspServer` is the kit's,
 * transcribed from the field table: "Each server config is a strict object with these fields. An
 * unknown key fails validation."
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/lsp_config.rs`, docs/RUST.md).
 */

const call = coreModule('format/lsp-config');

/** `stdio` (default) or `socket`: "Claude Code accepts `socket` but runs every server over stdio". */
export type LspTransport = (typeof LSP_TRANSPORTS)[number];

export interface LspServer {
  /** Language server binary: no whitespace unless the value starts with `/`. */
  readonly command: string;
  /** File extension (with its dot) to LSP language ID; at least one entry. */
  readonly extensionToLanguage: Readonly<Record<string, string>>;
  readonly args?: readonly string[];
  readonly transport?: LspTransport;
  readonly env?: Readonly<Record<string, string>>;
  /** Sent in the initialize request. Any JSON value. */
  readonly initializationOptions?: unknown;
  /** Sent by `workspace/didChangeConfiguration`. Any JSON value. */
  readonly settings?: unknown;
  readonly workspaceFolder?: string;
  /** Milliseconds, a positive integer. */
  readonly startupTimeout?: number;
  /** Milliseconds, a positive integer; unset means no timeout. */
  readonly shutdownTimeout?: number;
  /** Milliseconds, a positive integer; Claude Code defaults to 60000. */
  readonly requestTimeout?: number;
  /** Claude Code defaults to `true`. */
  readonly restartOnCrash?: boolean;
  /** Restart attempts before giving up, zero or more. */
  readonly maxRestarts?: number;
  /** Claude Code defaults to `true`. */
  readonly diagnostics?: boolean;
}

/** The fourteen rows of the table. */
export const LSP_FIELDS: readonly [
  'command',
  'extensionToLanguage',
  'args',
  'transport',
  'env',
  'initializationOptions',
  'settings',
  'workspaceFolder',
  'startupTimeout',
  'shutdownTimeout',
  'requestTimeout',
  'restartOnCrash',
  'maxRestarts',
  'diagnostics',
] = call('LSP_FIELDS');

export interface LspServerRead {
  /** Undefined when Claude Code would not load the entry. */
  readonly server: LspServer | undefined;
  readonly issues: readonly PluginIssue[];
}

/** One entry of a map; `origin` is the file (or `lspServers` for an inline map) it came from. */
export const readLspServer = (name: string, raw: unknown, origin: string): LspServerRead =>
  call('readLspServer', name, raw, origin);

export interface LspConfigRead {
  readonly servers: Record<string, LspServer>;
  readonly issues: PluginIssue[];
}

/**
 * The servers of one map: a `.lsp.json` (or another `.json` file `lspServers` names) or an inline
 * map. Components.md: "The file maps each server name directly to its configuration, with no
 * wrapper object", and "When any entry is invalid, the whole file is skipped at load": one invalid
 * entry leaves no server from this map. An inline map never gets here invalid: it is part of the
 * manifest's schema (manifest-schema.ts), so a bad inline entry keeps the plugin from loading
 * (spec §9.1.1 item 3, J9.5).
 */
export const readLspConfig = (raw: unknown, origin: string): LspConfigRead =>
  call('readLspConfig', raw, origin);

/**
 * Joins the maps of a plugin in loading order: `.lsp.json` first, then each declared source
 * (manifest-reference: "A server name declared later replaces an earlier one"; components.md: the
 * manifest server replaces the `.lsp.json` one of the same name). Judgment: replacement is of the
 * whole entry, and a replaced name keeps its first position, so the registration order that decides
 * which server keeps a shared extension is the order names first appeared. Limitation: the result is
 * a plain object, so integer-like names (`"9"`, `"10"`) come first in ascending order whatever their
 * position (spec/judgments.md).
 */
export const mergeLspSources = (sources: readonly LspConfigRead[]): LspConfigRead =>
  call('mergeLspSources', sources);
