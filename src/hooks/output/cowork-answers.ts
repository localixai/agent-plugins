/**
 * Agent Plugins Extended §7.5.13 `PostToolUse`, `updatedToolOutput`: "In a Cowork cloud session the
 * client ignores it for `AskUserQuestion` and `ExitPlanMode`, and Claude reads the original answer
 * or plan". Whether the session is one is the host's (`DispatchRequest.coworkCloud`).
 */

/** The tools whose answer or plan a Cowork cloud session keeps as the user gave it. */
const KEPT_IN_COWORK: ReadonlySet<unknown> = new Set<unknown>(['AskUserQuestion', 'ExitPlanMode']);

/** The merged effects without a hook's `updatedToolOutput` where the session keeps the original. */
export function withCoworkAnswers<
  M extends { readonly effects: { readonly updatedToolOutput?: unknown } },
>(merged: M, where: { readonly coworkCloud?: boolean; readonly facts: unknown }): M {
  if (where.coworkCloud !== true) return merged;
  if (!KEPT_IN_COWORK.has(Reflect.get(Object(where.facts), 'tool_name'))) return merged;
  if (merged.effects.updatedToolOutput === undefined) return merged;
  const { updatedToolOutput: _replaced, ...effects } = merged.effects;
  return { ...merged, effects };
}
