/**
 * The trust check before a background session starts or restarts (permissions.md, "Project allow
 * rules and workspace trust"; errors.md, "Workspace not trusted when dispatching a background
 * session"; Agent Plugins Extended §13.15 item 4). Pure over what the host knows: whether the
 * directory exists, whether it is trusted (`workspaceTrustState`), and whether a dialog can appear
 * (a terminal in the session's own directory).
 */

export interface BackgroundTrustFacts {
  /** The session's directory, as the message names it. */
  readonly directory: string;
  /** The directory was found on disk. */
  readonly resolved: boolean;
  /** It is the home directory, whose trust is never saved. */
  readonly isHome: boolean;
  /** The dialog was accepted for its key (or a covering parent) and is on record. */
  readonly trusted: boolean;
  /** A terminal in the session's own directory, where the dialog can appear. */
  readonly canShowDialog: boolean;
}

/** `start`: go ahead. `dialog`: show the trust dialog first and start once it is accepted. `refuse`: exit with `message`. */
export type BackgroundTrustVerdict =
  | { readonly verdict: 'start' }
  | { readonly verdict: 'dialog' }
  | { readonly verdict: 'refuse'; readonly message: string };

const refuse = (message: string): BackgroundTrustVerdict => ({ verdict: 'refuse', message });

/**
 * What happens before a background session runs in `facts.directory`. A directory not found on
 * disk is refused (`<path> could not be resolved on disk`); a trusted one starts, the home directory
 * never counting as trusted from an earlier session; else a terminal there shows the dialog, and
 * where none can appear the command exits (`Workspace not trusted. Run \`claude\` in <path> once and
 * accept the trust prompt, then retry.`, or `The home directory is trusted one session at a time`).
 */
export function backgroundSessionTrust(facts: BackgroundTrustFacts): BackgroundTrustVerdict {
  if (!facts.resolved) return refuse(`${facts.directory} could not be resolved on disk`);
  if (facts.trusted && !facts.isHome) return { verdict: 'start' };
  if (facts.canShowDialog) return { verdict: 'dialog' };
  return refuse(
    facts.isHome
      ? 'The home directory is trusted one session at a time'
      : `Workspace not trusted. Run \`claude\` in ${facts.directory} once and accept the trust prompt, then retry.`,
  );
}
