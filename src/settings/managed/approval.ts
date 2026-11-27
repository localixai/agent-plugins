/** Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/approval.rs`, docs/RUST.md). */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/managed/approval');

/**
 * Which delivered settings need the user's approval before they apply (server-managed-settings.md,
 * "Security approval dialogs" and "Environment variables and the approval dialog"). Pure: the host
 * shows the dialog; the kit says what it must name.
 */
export type ApprovalKind =
  | 'shell-command'
  | 'sandbox-binary'
  | 'sandbox-network-isolation'
  | 'custom-env'
  | 'hook';

export interface ApprovalItem {
  readonly kind: ApprovalKind;
  /** The setting (or `env.NAME`) the dialog names. */
  readonly key: string;
}

type Block = Readonly<Record<string, unknown>>;

/**
 * The items of delivered settings that need approval, in a stable order. `claudeMd` is not one:
 * "it's instruction text for Claude rather than a command Claude Code runs".
 */
export const approvalItems = (settings: Block): readonly ApprovalItem[] =>
  call('approvalItems', settings);
