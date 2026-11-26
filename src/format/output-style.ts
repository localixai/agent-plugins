import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';

/**
 * A custom output style (output-styles.md, "Create a custom output style"): a Markdown file whose
 * frontmatter is metadata and whose body is the instructions Claude Code sends with every request.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/output_style.rs`, docs/RUST.md).
 */

const call = coreModule('format/output-style');

export interface OutputStyle {
  /** `name`, else the file name without `.md`. */
  readonly name: string;
  /**
   * What `/output-style` and the `outputStyle` setting call it: `<plugin>:<name>` for a plugin style
   * (plugins/components.md, "Output styles"), `name` for a user, project or managed style.
   */
  readonly id: string;
  readonly description?: string;
  /** `keep-coding-instructions`, default `false`. */
  readonly keepCodingInstructions: boolean;
  /** `force-for-plugin`, default `false`, and always `false` for a style that is not a plugin's. */
  readonly forceForPlugin: boolean;
  /** The Markdown body: the instructions for Claude. */
  readonly instructions: string;
  /** The path as the host gave it. */
  readonly path: string;
}

/** Where the file was found. Only a plugin style is prefixed and may be forced. */
export interface OutputStyleContext {
  /** The plugin the file ships in; absent for a user, project or managed style. */
  readonly plugin?: string;
}

export interface OutputStyleRead {
  readonly style: OutputStyle;
  readonly issues: readonly PluginIssue[];
}

/**
 * Read one output style file. Never throws and never refuses a file (output-styles.md: "If the
 * YAML doesn't parse, the style still loads under its file name with no fields set"; "A misspelled
 * field is ignored without an error"). What the docs leave open is in spec/judgments.md: a YAML
 * error and a value of the wrong shape are warnings, never errors.
 */
export const readOutputStyle = (
  path: string,
  text: string,
  context: OutputStyleContext = {},
): OutputStyleRead => call('readOutputStyle', path, text, context);

/**
 * Of one plugin's styles that give the same `name`, exactly one is kept: the one whose path
 * relative to the plugin root sorts last by UTF-16 code unit (spec J6.7, oracle 2.1.325: of
 * `terse.md` and `terse2.md` both named `Terse`, `terse2.md` is listed). A fixed order keeps the
 * result independent of the file system's. The kept style stays where it was.
 */
export const lastPerName = (styles: readonly OutputStyle[]): OutputStyle[] =>
  call('lastPerName', styles);
