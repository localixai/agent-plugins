import type { AgentSource } from '../../format/agent-record.ts';
import type { PluginIssue } from '../../format/issues.ts';
import type { ScopeContext } from '../../foundation/scope-folders.ts';
import type { AgentsOrigin } from '../../settings/customization/sources.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { CatalogAgent } from './types.ts';

/**
 * Reads the agents folders through the `FolderScan` port (sub-agents.md, "Choose the subagent scope":
 * managed, project, user and `--add-dir` `.claude/agents/`, scanned "recursively, so you can organize
 * definitions into subfolders"). Nothing is resolved here: two files that declare one name are both
 * returned with where each came from, for `resolveAgents` to choose.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/catalog/scan.rs`, docs/RUST.md).
 */

const call = coreModule('agents/catalog/scan');

export interface AgentScanContext extends ScopeContext {
  /** Whether `strictPluginOnlyCustomization` lets a source load; absent, every source does. */
  readonly allows?: (origin: AgentsOrigin) => boolean;
  /** False for a session started with `--disable-slash-commands`, which watches nothing. Default true. */
  readonly watch?: boolean;
}

export interface ScannedAgent {
  readonly agent: CatalogAgent;
  /** The source of the file: `managed`, `project` or `user`. */
  readonly source: Extract<AgentSource, 'managed' | 'project' | 'user'>;
  /** Its folder's name for `strictPluginOnlyCustomization`; `addDir` for an added directory. */
  readonly origin: AgentsOrigin;
  /** The base directory the file was read below. */
  readonly base: string;
  /**
   * Where the file sits among the folders of its source: the nearer to the working directory, the
   * lower. "The definition closest to the working directory" wins.
   */
  readonly order: number;
  /** Where the file really lives; two entries with one target are one agent. */
  readonly canonical?: string;
}

export interface AgentScanOutcome {
  readonly entries: readonly ScannedAgent[];
  /** The files that were skipped with a reason, and the fields a file's frontmatter got wrong. */
  readonly issues: readonly PluginIssue[];
  /** The agents folders the session watches for changes: those that exist when it starts. */
  readonly watch: readonly string[];
}

/** Every agents folder of a session: managed, the project (and the added directories), personal. */
export const scanAgents = (context: AgentScanContext): Promise<AgentScanOutcome> =>
  call.async('scanAgents', context);
