/**
 * A marketplace managed settings declare under a name the user's registry holds from another source
 * (spec §16.31; plugins/troubleshooting.md): the client replaces the user's marketplace with the
 * declared one in an interactive session, but before that it reads `installed_plugins.json` and
 * reaches the declared source. While either fails it keeps the user's marketplace and says why.
 * Pure; the host checks the source and the file and does the replacing.
 */

export interface TakeBackFacts {
  readonly name: string;
  /** The declared source answered (cloned, downloaded or read). */
  readonly declaredReachable: boolean;
  /** `installed_plugins.json` was read. */
  readonly recordsReadable: boolean;
  /** The session is interactive: only such a session replaces the marketplace. */
  readonly interactive: boolean;
}

export type TakeBack =
  | { readonly replace: true }
  | { readonly replace: false; readonly message?: string };

/** Whether to replace the user's marketplace now, or the line that says why it stays. */
export function managedTakeBack(facts: TakeBackFacts): TakeBack {
  if (!facts.recordsReadable) {
    return {
      replace: false,
      message: `The list of installed plugins (installed_plugins.json) could not be read, so marketplace "${facts.name}", which managed settings declare, cannot be taken back from another source`,
    };
  }
  if (!facts.declaredReachable) {
    return {
      replace: false,
      message: `Marketplace "${facts.name}", which managed settings declare, stands from another source and cannot be taken back while its declared source is unreachable`,
    };
  }
  return facts.interactive ? { replace: true } : { replace: false };
}
