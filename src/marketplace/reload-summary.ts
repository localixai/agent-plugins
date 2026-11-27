import { coreModule } from '../wasm/bridge.ts';

/**
 * `/reload-plugins` (plugins/cli-reference.md, "/reload-plugins", "Reload summary", "Reloads that
 * change MCP tools", "Sessions without an interactive terminal"; plugins/troubleshooting.md, `Run
 * /reload-plugins to apply.`): the one line it prints, whether the reload is applied or refused,
 * and who may run it. Pure texts and decisions over counts the host measured.
 *
 * Judgments (spec/judgments.md): the clauses are joined to the counts by a space, dependencies
 * before errors, with no pluralization (the docs write `N plugins`, `N errors`); several servers
 * are listed in the order added, then removed, joined by `, `; when a reload changes both an MCP
 * server and the `LSP` tool the line names the MCP servers.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/reload_summary.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/reload-summary');

export interface ReloadCounts {
  readonly plugins: number;
  /** `skillCount` of every plugin's skills. */
  readonly skills: number;
  /** The agents loaded in the session, "including ones that don't come from plugins". */
  readonly agents: number;
  readonly hooks: number;
  readonly mcpServers: number;
  readonly lspServers: number;
  /** Plugins that failed to load. */
  readonly errors: number;
  /** Plugin dependencies this reload installed because they were missing. */
  readonly resolvedDependencies?: readonly string[];
}

/** "The skills count covers every skill a plugin provides, both its `commands/` entries and its `SKILL.md` skills." */
export const skillCount = (plugin: {
  readonly commands: number;
  readonly skillFiles: number;
}): number => call('skillCount', plugin);

export const reloadSummary = (
  counts: ReloadCounts,
  session: { readonly interactive: boolean },
): string => call('reloadSummary', counts, session);

// ---- the prompt cache ----

export interface ReloadChange {
  readonly addedMcp: readonly string[];
  readonly removedMcp: readonly string[];
  /** The reload adds or removes the `LSP` tool. */
  readonly lsp: 'adds' | 'removes' | 'unchanged';
  /** The change would invalidate the prompt cache. */
  readonly invalidatesPromptCache: boolean;
}

export type ReloadGate =
  | { readonly apply: true; readonly mcpDeferred: boolean }
  | { readonly apply: false; readonly message: string };

/**
 * "When the reload would add or remove a plugin MCP server or the `LSP` tool, and that change would
 * invalidate the prompt cache, Claude Code doesn't apply the reload" and prints `This reload changes
 * MCP tools (<server>) — your next message will re-read the whole conversation instead of using the
 * cache. Run /reload-plugins --force to apply.`; `--force` applies it anyway. Without an
 * interactive terminal "the reload doesn't connect or disconnect plugin MCP servers. Those changes
 * take effect in your next session", so it is applied with the servers deferred.
 */
export const reloadGate = (
  change: ReloadChange,
  options: { readonly force: boolean; readonly interactive: boolean },
): ReloadGate => call('reloadGate', change, options);

/** `/reload-plugins [--force]`; "`force` without dashes works too". */
export const parseReloadArguments = (text: string): { readonly force: boolean } =>
  call('parseReloadArguments', text);

/**
 * In a session without an interactive terminal the command runs "only when you type it into the
 * session yourself"; when it arrives another way it replies and reloads nothing. An interactive
 * terminal runs it whichever way it arrives (the docs restrict only the other sessions).
 */
export const reloadAllowed = (session: {
  readonly interactive: boolean;
  readonly typedByUser: boolean;
}): { readonly allowed: true } | { readonly allowed: false; readonly reply: string } =>
  call('reloadAllowed', session);
