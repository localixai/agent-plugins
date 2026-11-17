/**
 * The text after `"<name>" was not uninstalled: ` when a settings file of the scope still switches,
 * or may still switch, the plugin on (errors.md, "Plugin was not uninstalled").
 */

/**
 * Why a settings file of the scope may still switch the plugin on (errors.md, "Plugin was not
 * uninstalled"): `still-on` the write reported success but the entry is still there; `change-failed`
 * the file could not be saved, for `error`; `unreadable` it exists but is not readable as settings;
 * `not-read` it is on a network path or a link to one, or its path could not be examined.
 */
export type StillSwitchedOn =
  | { readonly file: string; readonly cause: 'still-on' | 'unreadable' | 'not-read' }
  | { readonly file: string; readonly cause: 'change-failed'; readonly error: string };

const TAKE_IT_OUT =
  'It is still installed. Take it out of "enabledPlugins" in that file yourself, then uninstall it again.';

// Judgment: for a file that was not read the last sentence also offers repairing it (errors.md,
// "What to do"); the docs quote only the switched-on form's (spec/judgments.md).
const REPAIR_IT =
  'It is still installed. Repair or replace that file, or take the plugin out of "enabledPlugins" in it yourself, then uninstall it again.';

const STILL_ON_TEXT: {
  readonly [cause in StillSwitchedOn['cause']]: (fact: StillSwitchedOn) => string;
} = {
  'still-on': (fact) =>
    `it is still switched on in ${fact.file}, although the settings change reported no error. ${TAKE_IT_OUT}`,
  'change-failed': (fact) =>
    `it is still switched on in ${fact.file}, and the settings change failed (${'error' in fact ? fact.error : ''}). ${TAKE_IT_OUT}`,
  unreadable: (fact) => `${fact.file} is there and could not be read. ${REPAIR_IT}`,
  'not-read': (fact) =>
    `${fact.file} (not read: it is on a network path or is a link to one, or could not be checked). ${REPAIR_IT}`,
};

/** The cause and the sentence that says what to do, for one settings file. */
export const stillOnReason = (fact: StillSwitchedOn): string => STILL_ON_TEXT[fact.cause](fact);
