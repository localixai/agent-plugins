import { coreModule } from '../wasm/bridge.ts';
import type { OutputStyle } from './output-style.ts';

/**
 * Which output style is active, and what that means for the system prompt (output-styles.md,
 * "Change your output style" and "How output styles work"). Pure: the host reads its settings and
 * passes the values in.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/output_style_select.rs`,
 * docs/RUST.md). A selection holds the host's own style object: the core names it by its place in
 * `styles`, and `withStyle` puts the object back.
 */

const call = coreModule('format/output-style-select');

interface BuiltinStyle {
  readonly id: string;
  readonly description: string;
}

/**
 * `default` and the four built-in styles, in the order output-styles.md lists them. The
 * descriptions are the docs' own words ("What changes" column; Default section).
 */
export const BUILTIN_OUTPUT_STYLES: readonly BuiltinStyle[] = call('BUILTIN_OUTPUT_STYLES');

/** The id Default has in the `/output-style` list. */
export const DEFAULT_OUTPUT_STYLE_ID: 'default' = call('DEFAULT_OUTPUT_STYLE_ID');

/** The style in force: Default, a built-in one, or a custom one (`style` is set). */
export interface StyleSelection {
  readonly id: string;
  /**
   * Whether Claude Code's software engineering instructions stay in the prompt: Default and the
   * built-in styles keep them ("Each of the four other built-in styles keeps those
   * instructions"), a custom style only with `keep-coding-instructions`.
   */
  readonly keepCodingInstructions: boolean;
  readonly style?: OutputStyle;
}

/** A selection as the core answers it: a custom style by its place in `styles`. */
interface Selected {
  readonly id: string;
  readonly keepCodingInstructions: boolean;
  readonly at?: number;
}

/** The selection with the host's own style object at the place the core named. */
function withStyle(styles: readonly OutputStyle[], { at, ...selection }: Selected): StyleSelection {
  const style = at === undefined ? undefined : styles[at];
  return style === undefined ? selection : { ...selection, style };
}

export interface OutputStyleRequest {
  /** The `outputStyle` setting, as the merged settings hold it. */
  readonly setting?: string;
  /** The argument of `/output-style <style>` given in this session. */
  readonly command?: string;
  /**
   * The custom styles of the enabled plugins, the user, the project and managed settings, in
   * precedence order: when two share an id the first wins, so a project style closer to the working
   * directory goes first, and of several forced styles the first loaded wins.
   */
  readonly styles: readonly OutputStyle[];
}

/**
 * The style the argument of `/output-style <style>` names, a built-in or a custom one, without
 * regard to case; `undefined` when it names none, which leaves the choice to the setting.
 */
export function findOutputStyle(
  styles: readonly OutputStyle[],
  argument: string,
): StyleSelection | undefined {
  const found: Selected | undefined = call('findOutputStyle', styles, argument);
  return found === undefined ? undefined : withStyle(styles, found);
}

/**
 * The active style. A `force-for-plugin` style wins over everything ("Overrides the user's
 * `outputStyle` setting"). Otherwise `/output-style` (which ignores case) wins over the setting,
 * which is case-sensitive: "A value that doesn't match a style name exactly, such as
 * `explanatory`, gives you the Default style."
 */
export const resolveOutputStyle = (request: OutputStyleRequest): StyleSelection =>
  withStyle(request.styles, call('resolveOutputStyle', request));

/** Which system prompt Claude Code builds. */
export type PromptKind = 'full' | 'short';

/**
 * Whether the final prompt carries Claude Code's software engineering instructions. On the full
 * prompt a custom style drops them unless `keep-coding-instructions` is true; on the short prompt
 * there is nothing to keep or drop.
 */
export const includeCodingInstructions = (
  selection: StyleSelection,
  promptKind: PromptKind,
): boolean => call('includeCodingInstructions', selection, promptKind);

export interface OutputStyleChoice {
  readonly id: string;
  readonly name: string;
  readonly description?: string;
  /** The style in force, which `/output-style` with no argument marks. */
  readonly current: boolean;
}

/**
 * What `/output-style` lists with no argument and the `/config` picker shows: Default, the
 * built-in styles, then each custom style with its `name` and `description` (else the first
 * non-empty line of its body: spec J6.3, oracle 2.1.325). It lists the same pool
 * `resolveOutputStyle` selects from, so a custom style that reuses a built-in id or an earlier
 * style's id is left out (the first wins) and exactly one row is current.
 */
export const listOutputStyles = (
  styles: readonly OutputStyle[],
  current: StyleSelection,
): readonly OutputStyleChoice[] => call('listOutputStyles', styles, current);
