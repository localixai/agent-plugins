/**
 * The parts of an LSP server's runtime the kit decides as pure functions over data
 * (plugins/manifest-reference.md, "lspServers"; plugins/components.md, "LSP servers"). Starting,
 * stopping and timing the process, and speaking the protocol, are the host's LSP client.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/lsp_runtime.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../../format/issues.ts';
import type { LspServer } from '../../format/lsp-config.ts';

const call = coreModule('loading/resolve/lsp-runtime');

export type RestartDecision = 'restart' | 'stay-stopped';

/**
 * Whether a crashed server is restarted. `restartOnCrash` false "leave[s] a crashed server stopped";
 * `maxRestarts` is the "restart attempts before giving up, zero or more". `restartsSoFar` counts the
 * attempts already made for this server (judgment: not the crash in hand, so with `maxRestarts: 2`
 * the first two crashes restart and the third does not). With `maxRestarts` unset, restarts are
 * unbounded (judgment J-L2: the docs name no default).
 */
export const lspRestartDecision = (
  server: Pick<LspServer, 'restartOnCrash' | 'maxRestarts'>,
  restartsSoFar: number,
): RestartDecision => call('lspRestartDecision', server, restartsSoFar);

/**
 * The part of the `initialize` request params this config decides: "Options sent in the initialize
 * request". The host adds its own (process ID, root, capabilities) around it.
 */
export const initializeOptionsOf = (
  server: Pick<LspServer, 'initializationOptions'>,
): { initializationOptions?: unknown } => call('initializeOptionsOf', server);

export interface DidChangeConfiguration {
  readonly method: 'workspace/didChangeConfiguration';
  readonly params: { readonly settings: unknown };
}

/**
 * The notification that carries `settings` ("Settings sent by `workspace/didChangeConfiguration`").
 * Judgment: a server without `settings` is sent none.
 */
export const didChangeConfigurationOf = (
  server: Pick<LspServer, 'settings'>,
): DidChangeConfiguration | undefined => call('didChangeConfigurationOf', server);

type Routed = Pick<LspServer, 'extensionToLanguage'>;

export interface LanguageRoute {
  readonly server: string;
  readonly languageId: string;
}

/**
 * The server and language ID for a file: "each file extension gets one server" and "the first
 * registered handles those files". `servers` is in registration order (the order of the record) and
 * its key must be unique across every plugin, so qualify it (`plugin:server`): two plugins that
 * each declare `go` would otherwise collide in the record and one would vanish.
 * Judgment: extensions match as written (case-sensitive) at the end of the last path segment, a name
 * that is only the extension (`.ts`) has none, and the longest matching extension wins (`.d.ts` over
 * `.ts`), the first registered server among equals.
 */
export const languageIdFor = (
  servers: Readonly<Record<string, Routed>>,
  filePath: string,
): LanguageRoute | undefined => call('languageIdFor', servers, filePath);

/**
 * The warning components.md quotes for a server that loses an extension to an earlier one:
 * `LSP server "<name>" is not used for <ext> files`. One per server and extension. `<name>` is the
 * record key, so it is plugin-qualified (`plugin:server`) when the servers come from several plugins.
 */
export const lspExtensionConflicts = (servers: Readonly<Record<string, Routed>>): PluginIssue[] =>
  call('lspExtensionConflicts', servers);
