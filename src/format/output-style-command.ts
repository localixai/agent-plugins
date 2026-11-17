import type { OutputStyle } from './output-style.ts';
import {
  BUILTIN_OUTPUT_STYLES,
  findOutputStyle,
  listOutputStyles,
  type OutputStyleChoice,
  type StyleSelection,
} from './output-style-select.ts';

/**
 * `/output-style` and `/config outputStyle=<style>` with the two refusals of spec §6.2 item 4
 * (errors.md, "Custom output styles can't be selected over Remote Control" and "Output styles are
 * saved to local settings which this session doesn't load"). Pure: the host says where the turn
 * came from and which setting sources the session loads, then shows the reply and, for
 * `selected`, writes `outputStyle` to the project's `.claude/settings.local.json`.
 */

/** The notice a turn over Remote Control or a relayed message gets beside a list or an unknown name. */
export const CUSTOM_STYLES_NOT_SELECTABLE =
  "Custom output styles can't be selected over Remote Control or from a relayed message. Select one in the session itself, or pick a built-in style here.";

/** The refusal of a selection in a session that does not load the `local` setting source. */
export const LOCAL_SETTINGS_NOT_LOADED =
  "Output styles are saved to local settings (.claude/settings.local.json), which this session doesn't load, so the style can't be changed here.";

export interface OutputStyleCommand {
  /** The argument of `/output-style <style>` or `/config outputStyle=<style>`; absent lists the styles. */
  readonly argument?: string;
  /** The custom styles in precedence order, as `resolveOutputStyle` takes them. */
  readonly styles: readonly OutputStyle[];
  /** The style in force, which the list marks. */
  readonly current: StyleSelection;
  /** The turn came over Remote Control or in a message relayed into the session: only built-in styles. */
  readonly relayed?: boolean;
  /** Whether the session's setting sources include `local` (`--setting-sources`, SDK `settingSources`); default true. */
  readonly loadsLocalSettings?: boolean;
}

/** What the command answers: a list, a selection to write, an unknown name, or a refusal. */
export interface OutputStyleReply {
  /** With no argument: the styles that can be selected, the current one marked. */
  readonly choices?: readonly OutputStyleChoice[];
  /** The style the argument names; the host writes its id as `outputStyle`. */
  readonly selected?: StyleSelection;
  /** The argument named no selectable style: the reply of an unknown name. */
  readonly unknown?: string;
  /** Nothing is selected and nothing written: the text the user reads. */
  readonly refusal?: string;
  /** Added to a list or an unknown-name reply over Remote Control or a relayed message. */
  readonly notice?: string;
}

const BUILTIN_IDS = new Set(BUILTIN_OUTPUT_STYLES.map(({ id }) => id));

/** The reply of `/output-style`, with or without an argument. */
export function outputStyleCommand(command: OutputStyleCommand): OutputStyleReply {
  const { argument, current, relayed = false } = command;
  const styles = relayed ? [] : command.styles;
  const notice = relayed ? { notice: CUSTOM_STYLES_NOT_SELECTABLE } : {};
  if (argument === undefined) {
    const choices = listOutputStyles(styles, current).filter(
      (choice) => !relayed || BUILTIN_IDS.has(choice.id),
    );
    return { choices, ...notice };
  }
  if (command.loadsLocalSettings === false) return { refusal: LOCAL_SETTINGS_NOT_LOADED };
  const selected = findOutputStyle(styles, argument);
  return selected === undefined ? { unknown: argument, ...notice } : { selected };
}
