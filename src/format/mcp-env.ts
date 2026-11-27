/**
 * `${VAR}` and `${VAR:-default}` in an MCP server's `command`, `args`, `env`, `url` and `headers`
 * (mcp.md, "Environment variable expansion in `.mcp.json`"). Pure over a lookup function, so it
 * serves a server of the local, project and user scopes (no plugin, no `SubstitutionScope`) and,
 * through `resolveEnvReference` and `splitDefault`, the plugin servers of `loading/resolve`: one rule
 * for both, so they cannot drift.
 *
 * Grammar (judgment, shared with `loading/resolve/substitute`): a reference is `${` text `}` with no
 * braces inside; the default ends at the first `}` and nests nothing; the name is a shell variable
 * name, and anything else is left as written.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_env.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';
import type { PluginMcpServer } from './mcp-config.ts';

const call = coreModule('format/mcp-env');

/** The environment as a function: a name to its value, `undefined` when it is unset. */
export type EnvLookup = (name: string) => string | undefined;

/** Every `${...}` in a text; the group is what is between the braces. */
export const BRACED_REFERENCE: RegExp = call('BRACED_REFERENCE');

export const isVariableName = (text: string): boolean => call('isVariableName', text);

/** `NAME` or `NAME:-default`. */
export const splitDefault = (inner: string): { name: string; fallback: string | undefined } =>
  call('splitDefault', inner);

/** The warning text for a reference that stayed as written (mcp.md, "Unset variables without a default"). */
export const missingVariableMessage = (name: string): string =>
  call('missingVariableMessage', name);

export interface EnvReferenceResolution {
  readonly text: string;
  /** True when the variable is unset with no default: `text` is the `${VAR}` as written. */
  readonly missing: boolean;
  /** True when a covered credential variable the environment has set read as empty. */
  readonly credentialRead: boolean;
}

/**
 * What one reference becomes. An empty string counts as set (mcp.md, "Variables set to an empty
 * string"): the default is for an unset variable only. A name `readsAsEmpty` covers is empty whether
 * or not it is set and its default is ignored, with no warning (mcp.md, "Credential variables that
 * read as empty").
 */
export const resolveEnvReference = (
  reference: { readonly name: string; readonly fallback: string | undefined },
  lookup: EnvLookup,
  readsAsEmpty?: (name: string) => boolean,
): EnvReferenceResolution => call('resolveEnvReference', reference, lookup, readsAsEmpty);

export interface EnvExpansion {
  readonly text: string;
  /** Variables left as written because they are unset and have no default, once each. */
  readonly missing: readonly string[];
  /** Covered credential variables that are set and were read as empty, once each. */
  readonly credentials: readonly string[];
}

/** Expands every reference of a text in one pass: a value is never scanned again. */
export const expandEnvText = (
  text: string,
  lookup: EnvLookup,
  readsAsEmpty?: (name: string) => boolean,
): EnvExpansion => call('expandEnvText', text, lookup, readsAsEmpty);

export interface McpExpansion {
  /** Undefined when the server cannot start: its `url` is not a URL after expansion. */
  readonly server: PluginMcpServer | undefined;
  /** A warning per field with an unset variable; an error when the server is dropped. */
  readonly issues: readonly PluginIssue[];
  /** The unset variables without a default, once each (the `claude mcp list` warning). */
  readonly missing: readonly string[];
  /** Debug-log lines for covered credentials the user set and a remote `url` or `headers` named. */
  readonly debug: readonly string[];
  /**
   * A remote server whose `url` is written empty: `not configured`, not connected and not an error
   * (mcp.md: "Claude Code doesn't attempt to connect to it"; spec §8.8 item 6). A `url` that becomes
   * empty or unusable only after expansion is still `'url' is not a valid URL`.
   */
  readonly notConfigured?: true;
}

/** What a host adds to the expansion of one server. */
export interface McpExpansionOptions {
  /**
   * More variable names that read as empty toward a remote server, on top of the kit's own list
   * (mcp.md, "Credential variables that read as empty"); it widens the list and never narrows it.
   * Judgment (spec/judgments.md): a host stricter than Claude Code withholds more of its environment.
   */
  readonly readsAsEmpty?: (name: string) => boolean;
}

/**
 * A server of the local, project or user scope with its references expanded (mcp.md, "Environment
 * variable expansion in `.mcp.json`"): `command`, `args` and `env` of a stdio server; `url` and
 * `headers` of a remote one (`headersHelper` is a command the process runs, so it is left as
 * written). An unset variable without a default keeps its text and is a warning; a covered
 * credential reads as empty in a remote server only.
 *
 * Judgment (spec/judgments.md): a `url` that does not parse after expansion drops the server, whatever
 * the cause, with the docs' `'url' is not a valid URL` (the docs show it for one that expands to
 * `/mcp`); a `url` written empty is a placeholder (`notConfigured`), not an error.
 */
export const expandMcpServer = (
  name: string,
  server: PluginMcpServer,
  lookup: EnvLookup,
  options: McpExpansionOptions = {},
): McpExpansion => call('expandMcpServer', name, server, lookup, options);

/** Where a configured server comes from, as far as what the surfaces show is concerned. */
export const MCP_DISPLAY_SCOPES: readonly ['local', 'project', 'user', 'managed'] =
  call('MCP_DISPLAY_SCOPES');
export type McpDisplayScope = (typeof MCP_DISPLAY_SCOPES)[number];

/**
 * What `/mcp`, `claude mcp list` and `claude mcp get` show of a server's URL or command line (mcp.md,
 * "How references appear in `/mcp` and CLI output"): for a server of the local, project or user
 * scope the `${VAR}` reference by name, never its resolved value; for a server the organization
 * provides, the URL's host only. Judgment: the host is the URL's host component (with its port) and
 * a URL that does not parse shows nothing of itself. A managed entry has no `command` (managed-mcp.md,
 * "What an entry can contain"), so a command line is shown as written in every scope.

 * `server` is the entry as written, never the result of `expandMcpServer`: the function shows what it
 * is given, so an expanded server would show the resolved value.
 */
export const displayReference = (server: PluginMcpServer, scope: McpDisplayScope): string =>
  call('displayReference', server, scope);

/** One header a surface shows: its name, and its value when the scope shows it. */
export interface DisplayedHeader {
  readonly name: string;
  readonly value?: string;
}

/**
 * What `claude mcp get` shows of a remote server's headers. For a server the organization provides:
 * "its header names without their values" (managed-mcp.md, "What users can see and change"). Judgment:
 * the docs give no rule for the other scopes, so a header there is shown as written (a `${VAR}`
 * reference by name, as for the URL; pass the entry as written, as for `displayReference`). A stdio
 * server has no headers.
 */
export const displayHeaders = (
  server: PluginMcpServer,
  scope: McpDisplayScope,
): readonly DisplayedHeader[] => call('displayHeaders', server, scope);
