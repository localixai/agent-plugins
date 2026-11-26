import type { PluginIssue } from '../../format/issues.ts';
import type { CommandSpec } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { ResolvedLspServer } from './lsp.ts';

/** Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/lsp_startup.rs`, docs/RUST.md). */
const call = coreModule('loading/resolve/lsp-startup');

export interface LspProcessPlan {
  readonly spec: CommandSpec;
  readonly cwd: string;
  readonly env: Readonly<Record<string, string>>;
  readonly transport: 'stdio';
  readonly startupTimeoutMs?: number;
  readonly shutdownTimeoutMs?: number;
}

export interface LspStartupPlan {
  readonly process?: LspProcessPlan;
  readonly issues: readonly PluginIssue[];
}

/** Process lookup is a host fact. The kit never downloads or starts the binary. */
export const planLspStartup = (
  server: ResolvedLspServer,
  facts: { readonly executableAvailable: boolean; readonly cwd: string },
): LspStartupPlan => call('planLspStartup', server, facts);
