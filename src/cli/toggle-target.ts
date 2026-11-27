/**
 * Which plugin and which scope `claude plugin enable` and `disable` act on (plugins/cli-reference.md,
 * "plugin enable", "plugin disable"): "Without `--scope`, the command checks your settings files in
 * the order local, project, user, and uses the first scope that mentions the plugin"; a `--scope`
 * where the plugin isn't declared "either writes an override or fails"; "If the plugin is already
 * enabled at the resolved scope, the command prints `Plugin "formatter" is already enabled` and exits
 * `1`. With `--json`, the result has `"failureCode": "already_in_goal_state"` and
 * `"alreadyInGoalState": true`". Pure over the graph facts: a scope "mentions" a plugin when its
 * `enabledPlugins` holds the id.
 *
 * Types only: the core runs it (`crates/agent-plugins/src/cli/toggle_target.rs`, docs/RUST.md), and
 * only the core calls it now; the plans and their callers share these shapes.
 */
import type { GraphFacts } from '../marketplace/dependency-graph.ts';

export interface ToggleArguments {
  /** `name` or `name@marketplace` (`name@synced` for a plugin synced from claude.ai). */
  readonly plugin: string;
  readonly scope?: string;
}

export interface ToggleFacts {
  readonly graph: GraphFacts;
  /** Plugin ids the organization requires (a required synced plugin cannot be disabled). */
  readonly requiredByOrganization?: ReadonlySet<string>;
}
