/**
 * The reasons a blocked model switch is reported with when no hook wrote one, and the report line
 * (errors.md, "Model switch was blocked by a PreModelSwitch hook"; Agent Plugins Extended §7.5.33).
 * The timeout reason is `timedOutBlocksSwitch` (notices.ts), carried by the hook's block.
 */
export const MODEL_SWITCH_REASONS = {
  /** The hook run ended without a verdict because a hook failed: never taken as approval. */
  failed: 'a PreModelSwitch hook failed before answering',
  /** The run ended without a verdict because the control stream closed. */
  cancelled: 'PreModelSwitch hooks were cancelled (the control stream closed) before answering',
} as const;

/**
 * `confirmation required, and this session cannot ask`: an `ask` without a reason where no
 * confirmation can be shown; `/model` in a `-p` run adds `(run /model interactively to confirm)`.
 */
export const cannotAskReason = (modelCommandInPrint = false): string =>
  `confirmation required, and this session cannot ask${modelCommandInPrint ? ' (run /model interactively to confirm)' : ''}`;

/**
 * The client cannot tell which `PreModelSwitch` hooks managed plugins deliver (a managed plugin
 * failed to load): the reason begins with what failed and ends with the fixed clause.
 */
export const managedHooksUncheckedReason = (whatFailed: string): string =>
  `${whatFailed}, so organization-managed PreModelSwitch hooks could not be checked`;

/**
 * The report: `Model switch to <model> was blocked by a PreModelSwitch hook: <reason>` for a switch
 * the user typed or picked; `Model switch blocked by a PreModelSwitch hook: <reason>`, not naming
 * the model, for one an Agent SDK host or Remote Control requested (`model` left out).
 */
export const modelSwitchBlocked = (reason: string, model?: string): string =>
  model === undefined
    ? `Model switch blocked by a PreModelSwitch hook: ${reason}`
    : `Model switch to ${model} was blocked by a PreModelSwitch hook: ${reason}`;
