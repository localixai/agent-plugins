import type { LspTransport } from './lsp-config.ts';

/**
 * The top-level keys of `.claude-plugin/plugin.json` and their documented types
 * (plugins/manifest-reference.md, "Fields"). Strict where the docs say so ("Unrecognized fields"):
 * `userConfig` options, `channels` entries, inline LSP configurations and monitor entries. Inline
 * hook and MCP configs are checked by their own readers, not here.
 *
 * Types only: the core runs it (`crates/agent-plugins/src/format/manifest_schema.rs`,
 * docs/RUST.md), and only the core calls it now, which holds the schemas; these are the types of
 * what passes them.
 */

type PathOrPaths = string | string[];
type FileOrInline = string | Record<string, unknown> | (string | Record<string, unknown>)[];

/** An option of `userConfig`, a strict object; the `options` rules hold too. */
export interface UserConfigOption {
  type: 'string' | 'number' | 'boolean' | 'directory' | 'file';
  title: string;
  description: string;
  required?: boolean | undefined;
  default?: string | number | boolean | string[] | undefined;
  options?: string[] | undefined;
  multiple?: boolean | undefined;
  sensitive?: boolean | undefined;
  min?: number | undefined;
  max?: number | undefined;
}

/** A `commands` entry of the object form: exactly one of `source` and `content`. */
export interface CommandEntry {
  source?: string | undefined;
  content?: string | undefined;
  description?: string | undefined;
  argumentHint?: string | undefined;
  model?: string | undefined;
  allowedTools?: string[] | undefined;
}

/**
 * A dependency: a non-empty string with at most one `@` (spec J16.17, oracle 2.1.325), or the
 * object form.
 */
export type PluginDependency =
  | string
  | { name: string; marketplace?: string | undefined; version?: string | undefined };

/** A `channels` entry: a channel bound to one of the plugin's MCP servers. */
export interface PluginChannel {
  server: string;
  displayName?: string | undefined;
  userConfig?: Record<string, UserConfigOption> | undefined;
}

/**
 * An inline LSP server configuration (plugins/manifest-reference.md, "lspServers"; spec §9.1.2):
 * unknown keys are refused by the schema, so the index signature only says the object is open
 * while it is read.
 */
interface InlineLspServer {
  [key: string]: unknown;
  command: string;
  extensionToLanguage: Record<string, string>;
  args?: string[];
  transport?: LspTransport;
  env?: Record<string, string>;
  initializationOptions?: unknown;
  settings?: unknown;
  workspaceFolder?: string;
  startupTimeout?: number;
  shutdownTimeout?: number;
  requestTimeout?: number;
  restartOnCrash?: boolean;
  maxRestarts?: number;
  diagnostics?: boolean;
}

type LspSources =
  | string
  | Record<string, InlineLspServer>
  | (string | Record<string, InlineLspServer>)[];

/** An inline monitor entry (plugins/manifest-reference.md, "monitors"; spec §9.2.1). */
interface InlineMonitor {
  [key: string]: unknown;
  name: string;
  command: string;
  description: string;
  when?: string;
}

type MonitorSources = string | InlineMonitor[];

/** A manifest that loaded: `name`, and every other documented key that passed its schema. */
export type PluginManifest = Readonly<{
  $schema?: string | undefined;
  name: string;
  displayName?: string | undefined;
  version?: string | undefined;
  description?: string | undefined;
  author?: { name: string; email?: string | undefined; url?: string | undefined } | undefined;
  homepage?: string | undefined;
  repository?: string | undefined;
  license?: string | undefined;
  keywords?: string[] | undefined;
  metadata?: Record<string, unknown> | undefined;
  icon?: string | undefined;
  documentationUrl?: string | undefined;
  supportUrl?: string | undefined;
  privacyPolicyUrl?: string | undefined;
  termsOfServiceUrl?: string | undefined;
  defaultEnabled?: boolean | undefined;
  dependencies?: PluginDependency[] | undefined;
  settings?: Record<string, unknown> | undefined;
  userConfig?: Record<string, UserConfigOption> | undefined;
  types?: string | undefined;
  channels?: PluginChannel[] | undefined;
  skills?: PathOrPaths | undefined;
  commands?: PathOrPaths | Record<string, CommandEntry> | undefined;
  agents?: PathOrPaths | undefined;
  hooks?: FileOrInline | undefined;
  mcpServers?: FileOrInline | undefined;
  lspServers?: LspSources | undefined;
  outputStyles?: PathOrPaths | undefined;
  workflows?: PathOrPaths | undefined;
  experimental?:
    | {
        themes?: PathOrPaths | undefined;
        monitors?: MonitorSources | undefined;
        evals?: PathOrPaths | undefined;
      }
    | undefined;
  // Moved under `experimental`; still loaded at the top level, with a validator warning.
  themes?: PathOrPaths | undefined;
  monitors?: MonitorSources | undefined;
}>;
