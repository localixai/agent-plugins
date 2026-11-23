import type { InstallScope } from '../marketplace/install-scopes.ts';
import {
  commandFailed,
  commandSucceeded,
  type CommandOutcome,
  type CommandOutput,
} from './result.ts';

/**
 * The end of `claude plugin install` for a plugin whose marketplace entry installs it by running a
 * command (spec §15.6.6, §17.4.2), once the host ran it: `--json` carries `commandPrinted` and
 * `commandFailed` (§17.2.2, 2.1.330). A command that wrote to stderr on a successful install turns
 * the human line into a warning that shows what it wrote; a command that exited with an error, was
 * killed or timed out fails the install. The wording and the `command_failed` code are the kit's
 * (spec/judgments.md).
 */

/** What the host saw of the command's run. */
export interface CommandSourceRan {
  /** What the command wrote to stderr; empty or absent when nothing. */
  readonly stderr?: string;
  /**
   * Why the command failed, when it exited with an error, was killed by a signal or timed out (the
   * message of the issue `runCommandSource` returned). Absent when it succeeded.
   */
  readonly failure?: string;
}

/** The success `planInstall` returned for the plugin, before its command ran. */
export interface PlannedCommandInstall {
  readonly output: CommandOutput;
  readonly effects: { readonly id: string; readonly scope: InstallScope };
}

const SUCCEEDED: { readonly [O in CommandOutcome]: boolean } = { ok: true, failed: false };

const restOf = (message: string): readonly string[] => message.split('\n').slice(1);

/** The planned success with the two booleans, or the failure the command caused. */
export function concludeCommandInstall(
  planned: PlannedCommandInstall,
  ran: CommandSourceRan,
): CommandOutput {
  if (!('result' in planned.output)) return planned.output;
  const { result } = planned.output;
  if (!SUCCEEDED[result.outcome]) return planned.output;
  const { id, scope } = planned.effects;
  const fields = result.fields ?? {};
  if (ran.failure !== undefined) {
    return commandFailed('install', `Failed to install plugin "${id}": ${ran.failure}`, {
      failureCode: 'command_failed',
      fields: { ...fields, commandPrinted: false, commandFailed: true },
    });
  }
  const printed = (ran.stderr ?? '').trimEnd();
  if (printed === '') {
    return commandSucceeded('install', result.message, {
      fields: { ...fields, commandPrinted: false, commandFailed: false },
    });
  }
  const warning = [
    `Installed ${id} (scope: ${scope}) with a warning: its install command wrote to stderr:`,
    printed,
    ...restOf(result.message),
  ].join('\n');
  return commandSucceeded('install', warning, {
    fields: { ...fields, commandPrinted: true, commandFailed: false },
  });
}
