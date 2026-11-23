import type { GraphFacts } from '../marketplace/dependency-graph.ts';
import { installClosure, type InstallStep } from '../marketplace/dependency-install-plan.ts';
import type { InstallScope } from '../marketplace/install-scopes.ts';
import type { Environment } from '../ports/environment.ts';
import { DISABLE_NONESSENTIAL_TRAFFIC, isSet } from '../updates/env-flags.ts';

/**
 * The dependencies `claude plugin update` installs (spec §16.24.1 item 4, §17.7): a declared
 * dependency the plugin is missing is installed under the limits of an install, except while
 * `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` is set; the ones left unmet are listed after `still
 * unresolved:`, with why none was installed when traffic is off (§16.30). The opening of that line
 * and the `(+ N dependencies: …)` list are the kit's (spec/judgments.md).
 */

/** "The success message lists them": ` (+ N dependencies: <ids>)` (wording: spec/judgments.md). */
export const dependencyList = (steps: readonly InstallStep[]): string =>
  steps.length === 0
    ? ''
    : ` (+ ${steps.length} dependencies: ${steps.map((step) => step.id).join(', ')})`;

const NO_TRAFFIC_LINE = 'Dependencies are not installed while non-essential traffic is turned off.';

/** What an update does about the plugin's dependencies. */
type UpdateDependencies =
  /** The catalog plugins to fetch before planning again (as `InstallPlan.needs`). */
  | { readonly needs: readonly string[] }
  | {
      /** The dependencies to install with the update; empty when none is. */
      readonly steps: readonly InstallStep[];
      /** The lines added after the success line. */
      readonly lines: readonly string[];
    };

const NOTHING: UpdateDependencies = { steps: [], lines: [] };

const idOfIssue = (path: string): string => path.replace(/^dependencies\./, '');

const unresolvedLine = (ids: readonly string[]): readonly string[] =>
  ids.length === 0 ? [] : [`Dependencies still unresolved: ${[...new Set(ids)].join(', ')}`];

/** The dependencies of `id` the update installs, or names as unresolved; nothing without `graph`. */
export function updateDependencies(
  target: { readonly id: string; readonly scope: InstallScope },
  graph: GraphFacts | undefined,
  env: Environment,
): UpdateDependencies {
  if (graph === undefined || graph.catalog[target.id] === undefined) return NOTHING;
  const closure = installClosure(
    { root: target.id, scope: target.scope, operation: 'update' },
    graph,
  );
  const left = closure.leftOut.map((one) => one.id);
  if (isSet(env, DISABLE_NONESSENTIAL_TRAFFIC)) {
    const unmet = [...closure.steps.map((step) => step.id), ...left];
    return unmet.length === 0
      ? NOTHING
      : { steps: [], lines: [...unresolvedLine(unmet), NO_TRAFFIC_LINE] };
  }
  if (closure.needs.length > 0) return { needs: closure.needs };
  const failed = closure.issues.map((issue) => idOfIssue(issue.path));
  return { steps: closure.steps, lines: unresolvedLine([...failed, ...left]) };
}
