/**
 * Whether dynamic workflows are on (settings-reference.md, `disableWorkflows`, `enableWorkflows`;
 * workflows.md, "Turn workflows off").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/workflows/enablement.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyView } from '../view.ts';

const call = coreModule('settings/workflows/enablement');

/** What the host tells the workflow keys besides the layers: plain data, the kit reads no `process.env`. */
export interface WorkflowsContext {
  /** The host's environment, for `CLAUDE_CODE_DISABLE_WORKFLOWS`. */
  readonly env: Readonly<Record<string, string | undefined>>;
  /**
   * The signed-in plan: workflows are off by default on Pro (settings-reference.md, `enableWorkflows`).
   * The host applies the release rule of `workflowSizeGuideline` (Claude Code v2.1.271 or later).
   */
  readonly plan: { readonly pro: boolean };
  /** The organization turned workflows off on the Claude Code admin settings page (workflows.md). */
  readonly orgWorkflowsOff?: boolean;
}

/** `CLAUDE_CODE_DISABLE_WORKFLOWS`, equivalent to `disableWorkflows` for one session (env-vars.md). */
export const WORKFLOWS_ENV: string = call('WORKFLOWS_ENV');

/** What can turn workflows off, in the order a decision lists them. */
export const WORKFLOWS_OFF_BY: readonly [
  'disableWorkflows',
  'env',
  'orgPolicy',
  'enableWorkflows',
  'plan',
] = call('WORKFLOWS_OFF_BY');
export type WorkflowsOffBy = (typeof WORKFLOWS_OFF_BY)[number];

/** What turned workflows on, when nothing turned them off. */
export type WorkflowsOnBy = 'enableWorkflows' | 'plan';

/**
 * What workflows take with them when they are off (workflows.md, "Turn workflows off"): `/workflows`,
 * the workflow commands and the `/workflow-authoring` skill are unavailable, the `ultracode` keyword
 * no longer triggers a run and the Ultracode toggle is removed from `/effort`. A run already in progress
 * keeps going: the kit holds no run state, so that is a duty of the host, which must not cancel a run
 * because this decision turned to off (spec/judgments.md).
 */
export interface WorkflowSurfaces {
  readonly workflowsView: boolean;
  readonly workflowCommands: boolean;
  readonly workflowAuthoringSkill: boolean;
  readonly keyword: boolean;
  readonly ultracodeToggle: boolean;
}

export interface WorkflowsDecision {
  readonly enabled: boolean;
  /** Empty when enabled; otherwise every reason that applies. Any one of them keeps workflows off. */
  readonly offBy: readonly WorkflowsOffBy[];
  /** Why they are on: an explicit `enableWorkflows: true`, or the plan's default. */
  readonly enabledBy: WorkflowsOnBy | undefined;
  readonly surfaces: WorkflowSurfaces;
  /** Whether `/config` shows its Dynamic workflows row. */
  readonly configRowVisible: boolean;
}

/**
 * Whether dynamic workflows are on: off by `disableWorkflows`, by `CLAUDE_CODE_DISABLE_WORKFLOWS=1`,
 * by the organization's policy or by `enableWorkflows: false`; otherwise on, except on the Pro plan
 * where they are off unless `enableWorkflows` is `true`. Nothing that turns them off can be undone by
 * `enableWorkflows: true` or by the other off switch (settings-reference.md, `disableWorkflows`,
 * `enableWorkflows`).
 */
export const workflowsDecision = (view: PolicyView, context: WorkflowsContext): WorkflowsDecision =>
  call('workflowsDecision', view, {
    env: context.env,
    plan: { pro: context.plan.pro },
    orgWorkflowsOff: context.orgWorkflowsOff,
  });
