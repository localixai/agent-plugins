import type { InstallStep } from '../marketplace/dependency-install-plan.ts';
import type { InstallScope } from '../marketplace/install-scopes.ts';
import {
  notAppliedLines,
  notYetSetLine,
  type ConfigOutcome,
  type InstallOptions,
} from './install-config.ts';
import { dependencyList } from './update-dependencies.ts';

/** What a successful `claude plugin install` reports on. */
export interface Succeeded {
  readonly id: string;
  readonly scope: InstallScope;
  readonly steps: readonly InstallStep[];
  readonly config: ConfigOutcome;
  readonly options: InstallOptions;
  readonly leftOut: readonly string[];
  /** The `engines` note of a local-folder plugin outside its range (§2.4); empty otherwise. */
  readonly engineNote: string;
}

/** The success lines: the install, what `--config` did not apply, the options still unset, dependencies left to the user. */
export function successLines(done: Succeeded): string {
  const unset = notYetSetLine(done.id, done.config);
  return [
    `Successfully installed plugin: ${done.id} (scope: ${done.scope})${done.engineNote}${dependencyList(done.steps)}`,
    ...notAppliedLines(done.config, done.options),
    ...(unset === undefined ? [] : [unset]),
    ...done.leftOut,
  ].join('\n');
}
