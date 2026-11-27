/**
 * The workflow size guideline in force (settings-reference.md, `workflowSizeGuideline`; workflows.md,
 * "Set a size guideline", "Cost").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/workflows/size.rs`, docs/RUST.md).
 */
import type { Settings } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyScope } from '../layers.ts';
import type { PolicyView } from '../view.ts';

const call = coreModule('settings/workflows/size');

export type WorkflowSize = NonNullable<Settings['workflowSizeGuideline']>;

/** What the host tells the size guideline besides the layers. */
export interface WorkflowSizeContext {
  /** Pro signed in: the default is `small` there (settings-reference.md, `workflowSizeGuideline`). */
  readonly plan: { readonly pro: boolean };
  /**
   * The Dynamic workflow size choice of `/config`, which Claude Code keeps in `~/.claude.json`: a file
   * the kit never reads, so the host hands the value over.
   */
  readonly configChoice?: WorkflowSize | undefined;
  /**
   * The session has ultracode on (`/effort ultracode`, or the `ultracode` setting): a session state the
   * host holds. The `Large workflow` warning does not show then (workflows.md, "Cost").
   */
  readonly ultracode?: boolean | undefined;
}

/** Where the size in force comes from. */
export type WorkflowSizeSource = 'settings' | 'config' | 'default';

export interface WorkflowSizeDecision {
  readonly size: WorkflowSize;
  readonly source: WorkflowSizeSource;
  /** The file that set it, when `source` is `settings`. */
  readonly settingsScope: PolicyScope | undefined;
  /** Claude aims for fewer agents than this; none for `unrestricted`. */
  readonly agentsBelow: number | undefined;
  /** Advice sent to Claude, "not an enforced cap". */
  readonly enforced: false;
  /** Whether `/config` shows its Dynamic workflow size row: hidden while a settings file sets the key. */
  readonly configRowVisible: boolean;
  /**
   * The agent count above which the progress line shows `Large workflow` (workflows.md, "Cost");
   * `undefined` while ultracode is on, which shows no warning.
   */
  readonly largeWorkflowThreshold: number | undefined;
  /**
   * The projected token total above which the progress line shows `Large workflow` (1.5 million);
   * `undefined` while ultracode is on. The host compares it with its run's projection: the kit holds
   * no run data (spec/judgments.md).
   */
  readonly largeWorkflowTokens: number | undefined;
}

/**
 * The size guideline in force: a value in a settings file, else the `/config` choice, else the default
 * (`medium`, `small` on Pro). A value of another shape was dropped when the file was read, so it never
 * reaches here. A size the user chose replaces the 25-agent threshold with its own agent count;
 * `unrestricted` has none, so it leaves 25 (spec/judgments.md). With ultracode on there is no warning,
 * so neither threshold is returned.
 */
export const workflowSize = (
  view: PolicyView,
  context: WorkflowSizeContext,
): WorkflowSizeDecision =>
  call('workflowSize', view, {
    plan: { pro: context.plan.pro },
    configChoice: context.configChoice,
    ultracode: context.ultracode,
  });
