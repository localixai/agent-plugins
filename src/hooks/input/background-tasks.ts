/**
 * The `background_tasks` and `session_crons` of a `Stop` or `SubagentStop` input (hooks.md, "Stop
 * input"): free text is "capped at 1000 characters with an in-string `… [+N chars]` marker when
 * clipped", and the fields only some task types have are "present only for" those types.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/input/background_tasks.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { EventFactsOf } from '../contracts.ts';

type StopFacts = EventFactsOf<'Stop'>;
export type BackgroundTask = NonNullable<StopFacts['background_tasks']>[number];
export type SessionCron = NonNullable<StopFacts['session_crons']>[number];

const call = coreModule('hooks/input/background-tasks');

/** Each task capped and stripped of the fields its type does not have; unknown types keep none. */
export const normalizeBackgroundTasks = (
  tasks: readonly BackgroundTask[],
  path: string,
): { readonly tasks: BackgroundTask[]; readonly issues: readonly PluginIssue[] } =>
  call('normalizeBackgroundTasks', tasks, path);

/** Each cron with its prompt capped. */
export const normalizeSessionCrons = (crons: readonly SessionCron[]): SessionCron[] =>
  call('normalizeSessionCrons', crons);
