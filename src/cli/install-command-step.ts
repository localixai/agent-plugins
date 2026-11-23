import type { CatalogPlugin } from '../marketplace/dependency-graph.ts';
import type { InstallScope } from '../marketplace/install-scopes.ts';
import type { Sha256 } from '../ports/marketplace.ts';
import {
  decideAcceptance,
  type CommandAcceptance,
  type DisplayedCommand,
  type ShownCommand,
} from './command-acceptance.ts';
import type { InstallFound, InstallRun } from './install-plan.ts';
import { displayedOf } from './install-lookup.ts';
import type { PendingInstall } from './install-start.ts';
import { commandFailed, type ResultFields } from './result.ts';

/**
 * The step of `claude plugin install` that stops at a displayed command (plugins/cli-reference.md,
 * "Accept a displayed install command"): a command-source or `headersHelper` entry prints its
 * command and needs `-y`, the answer to the question, or the digest `--accept-command` names.
 */

const shownFields = (shown: ShownCommand, id: string, scope: InstallScope): ResultFields => ({
  pluginId: id,
  scope,
  shownCommand: { ...shown },
});

/** A run that printed the command and did not run it. */
function notRun(
  acceptance: Extract<CommandAcceptance, { ask: false }>,
  context: {
    readonly id: string;
    readonly scope: InstallScope;
    readonly before: readonly string[];
    readonly command: string;
  },
): InstallRun {
  return {
    before: [...context.before, context.command],
    output: commandFailed('install', acceptance.line, {
      failureCode: 'command_not_accepted',
      fields: shownFields(acceptance.shownCommand, context.id, context.scope),
    }),
  };
}

const acceptanceOf = (
  shown: DisplayedCommand,
  pending: PendingInstall,
  found: InstallFound,
  sha256: Sha256,
): Promise<CommandAcceptance> =>
  decideAcceptance(
    {
      shown,
      catalog: found.catalog,
      flags: {
        ...(pending.yes ? { yes: true } : {}),
        ...(pending.acceptCommand === undefined ? {} : { acceptCommand: pending.acceptCommand }),
      },
      session: pending.session,
      tty: pending.tty,
      ...(found.answer === undefined ? {} : { answer: found.answer }),
    },
    sha256,
  );

interface Shown {
  readonly id: string;
  readonly entry: CatalogPlugin;
  readonly before: readonly string[];
}

/** The run that ends the install at its command: the question, or the refusal. `undefined` when the command runs or the entry has none. */
export async function stopAtCommand(
  at: Shown,
  pending: PendingInstall,
  found: InstallFound,
  sha256: Sha256,
): Promise<InstallRun | undefined> {
  const shown = displayedOf(at.id, at.entry);
  if (shown === undefined) return undefined;
  const acceptance = await acceptanceOf(shown, pending, found, sha256);
  if (acceptance.run) return undefined;
  if (acceptance.ask) return { before: [...at.before, shown.command], prompt: acceptance.prompt };
  return notRun(acceptance, {
    id: at.id,
    scope: pending.scope,
    before: at.before,
    command: shown.command,
  });
}
