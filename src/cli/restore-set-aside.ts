import { commandSucceeded, usageError, type CommandOutput } from './result.ts';

/**
 * `claude plugin restore-set-aside <plugin>@<marketplace> [--discard]` as a plan (spec §17.10.1):
 * when managed settings replaced a marketplace the user added, the client set aside the `userConfig`
 * options and secrets saved under that marketplace's ids; this command restores them under the id,
 * or deletes them with `--discard`. Only the full id the `claude plugin list` note gives is matched,
 * so a bare name never has anything set aside. Pure: the host knows which ids hold set-aside values
 * and carries out `action`.
 */

export interface RestoreSetAsideArguments {
  /** The full `name@marketplace` id. */
  readonly plugin: string;
  /** `--discard`: delete the set-aside values instead of restoring them. */
  readonly discard?: boolean;
  /** `--json`, which the command does not take (Judgment J17.34). */
  readonly json?: boolean;
}

export interface RestoreSetAsideFacts {
  /** The ids that hold set-aside options or secrets. */
  readonly setAside: readonly string[];
}

/** What the host does with the set-aside values of `id`. */
export interface SetAsideAction {
  readonly kind: 'restore' | 'discard';
  readonly id: string;
}

export interface RestoreSetAsidePlan {
  readonly output: CommandOutput;
  /** Absent when nothing changes. */
  readonly action?: SetAsideAction;
}

const LINES: { readonly [K in SetAsideAction['kind']]: (id: string) => string } = {
  restore: (id) => `Restored the options and secrets set aside for ${id}.`,
  discard: (id) => `Discarded the options and secrets set aside for ${id}.`,
};

export function planRestoreSetAside(
  args: RestoreSetAsideArguments,
  facts: RestoreSetAsideFacts,
): RestoreSetAsidePlan {
  if (args.json === true) return { output: usageError("error: unknown option '--json'") };
  if (!facts.setAside.includes(args.plugin)) {
    return {
      output: commandSucceeded(
        'restore-set-aside',
        `No options or secrets are set aside for ${args.plugin}.`,
      ),
    };
  }
  const action: SetAsideAction = {
    kind: args.discard === true ? 'discard' : 'restore',
    id: args.plugin,
  };
  return { output: commandSucceeded('restore-set-aside', LINES[action.kind](action.id)), action };
}

/**
 * The values an id holds after a restore: a value it already holds for an option is kept and the
 * set-aside one dropped, which §17.10.1 item 1 allows.
 */
export const restoredValues = <V>(
  current: Readonly<Record<string, V>>,
  setAside: Readonly<Record<string, V>>,
): Readonly<Record<string, V>> => ({ ...setAside, ...current });
