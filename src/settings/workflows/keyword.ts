/**
 * Whether typing `ultracode` in a prompt starts a dynamic workflow (workflows.md, "Where the keyword
 * works"; settings-reference.md, `workflowKeywordTriggerEnabled`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/workflows/keyword.rs`,
 * docs/RUST.md).
 */
import type { SDKMessageOrigin } from '../../protocol/official.ts';
import type { PolicyView } from '../view.ts';
import type { WorkflowsContext } from './enablement.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/workflows/keyword');

/**
 * How a prompt reaches the session (workflows.md, "Where the keyword works"): typed at the
 * interactive prompt, in an IDE extension panel or in a Remote Control client; sent by an Agent SDK
 * application; passed with `-p`; run as a scheduled task; or relayed from a webhook payload or a pull
 * request comment.
 */
export const PROMPT_SURFACES: readonly [
  'interactive',
  'ideExtension',
  'remoteControl',
  'sdk',
  'print',
  'scheduledTask',
  'relayedEvent',
] = call('PROMPT_SURFACES');
export type PromptSurface = (typeof PROMPT_SURFACES)[number];

export interface PromptInput {
  readonly surface: PromptSurface;
  readonly text: string;
  /** An SDK application's stamp on the prompt; read only for the `sdk` surface. */
  readonly origin?: SDKMessageOrigin;
  /** The person dismissed the highlight for this prompt (`Option+W`, `Alt+W`, or backspace after it). */
  readonly dismissed?: boolean;
}

/**
 * Why a prompt does not trigger a workflow, in the order the kit checks them (spec/judgments.md). The
 * setting is named only for a prompt that has the keyword: `/effort ultracode` reads the same either way.
 */
export type KeywordMiss =
  | 'workflowsOff'
  | 'notTypedByHuman'
  | 'dismissed'
  | 'keywordSettingOff'
  | 'noKeyword';

export interface KeywordVerdict {
  readonly triggers: boolean;
  /** Why not, when it does not trigger. */
  readonly reason: KeywordMiss | undefined;
  /** The keyword Claude Code highlights in the input, as a range of the text. */
  readonly highlight: { readonly start: number; readonly end: number } | undefined;
}

/**
 * Whether typing `ultracode` in `prompt` starts a dynamic workflow: workflows are on, the prompt is one
 * a person typed (settings-reference.md, `workflowKeywordTriggerEnabled`; workflows.md, "Where the keyword
 * works"), its highlight was not dismissed, the keyword is in the text and `workflowKeywordTriggerEnabled` is
 * not `false` (default `true`). The key switches the keyword only: `/effort ultracode`,
 * `/workflows` and saved workflow commands do not read it.
 */
export const keywordTrigger = (
  view: PolicyView,
  context: WorkflowsContext,
  prompt: PromptInput,
): KeywordVerdict =>
  call(
    'keywordTrigger',
    view,
    { env: context.env, plan: { pro: context.plan.pro }, orgWorkflowsOff: context.orgWorkflowsOff },
    prompt,
  );
