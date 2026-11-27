/**
 * What runs before a folder is trusted (permissions.md, "What runs before you trust a folder"): one
 * row of the docs' table per kind of repository content, with the two situations in which the folder
 * itself is not trusted: only a parent folder was trusted, or `claude -p` or the SDK ran there, which
 * never shows the trust dialog. Pure: the settings and MCP streams ask `contentGate`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/trust_gates.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('loading/trust-gates');

export const REPOSITORY_CONTENT: readonly [
  'settings-hooks',
  'env-and-helper-commands',
  'skill-hooks',
  'skill-allowed-tools',
  'allow-rules',
  'additional-directories',
  'subagent-frontmatter-hooks',
  'skills-dir-plugin',
  'extra-known-marketplaces',
  'subagent-inline-mcp',
  'mcp-json-servers',
  'headers-helper',
] = call('REPOSITORY_CONTENT');
export type RepositoryContent = (typeof REPOSITORY_CONTENT)[number];

/**
 * What a gate asks of the user before the content takes effect: nothing, the trust dialog (shown
 * again, naming the content), a per-server approval, or never (no dialog is offered).
 */
export type Consent = 'none' | 'trust-dialog' | 'server-approval' | 'never';

export interface ContentGate {
  /** The content takes effect now. */
  readonly applies: boolean;
  readonly consent: Consent;
  /** The line Claude Code prints to stderr, as the docs name it. */
  readonly stderr?: string;
  /** What is used meanwhile. */
  readonly fallback?: 'static-headers';
  /** Whether approvals the repository commits in its own settings count (`.mcp.json` servers). */
  readonly repositoryApprovals?: boolean;
}

/** The columns of the table, and the folder's other states. */
export const TRUST_SITUATIONS: readonly ['trusted', 'parent-trusted', 'non-interactive', 'dialog'] =
  call('TRUST_SITUATIONS');
export type TrustSituation = (typeof TRUST_SITUATIONS)[number];

type Row = { readonly [S in TrustSituation]: ContentGate };

/**
 * One row per kind of content. Judgment (spec/judgments.md): the table has no column for a folder
 * that nothing was trusted for in an interactive session (`dialog`), nor for a trusted one; there the
 * content that needs the exact folder waits for the dialog the session shows, hooks of settings
 * files do too (hooks.md, "Workspace trust"), and what the table calls "Used" in both columns, a
 * skill's `allowed-tools` (never gated "in any session"), is used.
 */
export const CONTENT_GATES: { readonly [C in RepositoryContent]: Row } = call('CONTENT_GATES');

export interface TrustSituationFacts {
  /** The trust dialog was accepted for this exact folder. */
  readonly folderTrusted: boolean;
  /** Only a parent folder was trusted. */
  readonly parentTrusted: boolean;
  /** `claude -p` or the SDK. */
  readonly nonInteractive: boolean;
  /** The folder is a git repository nested inside the trusted one (a submodule, a clone). */
  readonly nestedRepository: boolean;
}

/**
 * Which column applies. "The parent-folder column doesn't apply inside a nested repository: in an
 * interactive session Claude Code shows the trust dialog for it, and a `claude -p` or SDK run there
 * follows the `claude -p` column."
 */
export const trustSituation = (facts: TrustSituationFacts): TrustSituation =>
  call('trustSituation', facts);

export const contentGate = (content: RepositoryContent, situation: TrustSituation): ContentGate =>
  call('contentGate', content, situation);

/** The kinds of entry a project settings file's `permissions` holds, and whether each grants capability. */
export const PROJECT_PERMISSION_ENTRIES: readonly [
  'allow',
  'ask',
  'deny',
  'additionalDirectories',
] = call('PROJECT_PERMISSION_ENTRIES');
export type ProjectPermissionEntry = (typeof PROJECT_PERMISSION_ENTRIES)[number];

/**
 * "`permissions.allow` rules and `permissions.additionalDirectories` entries in a project's
 * `.claude/settings.json` grant capability, so Claude Code applies them only after you accept the
 * workspace trust dialog for that folder ... `deny` and `ask` rules aren't affected, since they only
 * restrict."
 */
export const projectEntryApplies = (
  entry: ProjectPermissionEntry,
  folderTrusted: boolean,
): boolean => call('projectEntryApplies', entry, folderTrusted);

/** What the dialog lists "so you can review them first": the rules and directories the folder would grant. */
export const trustDialogListing = (permissions: {
  readonly allow?: readonly string[] | undefined;
  readonly additionalDirectories?: readonly string[] | undefined;
}): { readonly allow: readonly string[]; readonly additionalDirectories: readonly string[] } =>
  call('trustDialogListing', permissions);
