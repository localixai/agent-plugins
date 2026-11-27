/**
 * What "Yes, and don't ask again" does after a prompt (permissions.md, "Permission system", the last
 * column of the table), per type of tool: how long the approval lasts and what it is keyed to. A
 * saved approval is a rule the host writes to `.claude/settings.local.json` at the root of the git
 * repository (resolved through worktrees to the main checkout); the host decides where it can.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/approval_memory.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('rules/decide/approval-memory');

export const TOOL_TYPES: readonly [
  'read-only',
  'bash',
  'file-modification',
  'web-fetch',
  'web-search',
] = call('TOOL_TYPES');
export type ToolType = (typeof TOOL_TYPES)[number];

/** How long an approval lasts. */
export type ApprovalLifetime = 'none' | 'session' | 'repository';

export interface ApprovalMemory {
  readonly lifetime: ApprovalLifetime;
  /** What a saved approval names: a command prefix, a domain, or the whole tool. */
  readonly keyedBy: 'command' | 'domain' | 'tool' | 'none';
}

export const toolTypeOf = (toolName: string): ToolType | undefined => call('toolTypeOf', toolName);

/** What a session can change about "Yes, and don't ask again". */
export interface ApprovalOptions {
  /**
   * `allowManagedPermissionRulesOnly` is on: it "hides the always-allow choices in permission
   * prompts, and stops saving new rules" (settings-reference.md), so an approval that would be saved
   * as a rule is not offered (spec/judgments.md).
   */
  readonly rulesLocked?: boolean;
}

/** What choosing "Yes, and don't ask again" does for a tool; `undefined` for a tool the table does not list. */
export const approvalMemory = (
  toolName: string,
  options: ApprovalOptions = {},
): ApprovalMemory | undefined => call('approvalMemory', toolName, options);
