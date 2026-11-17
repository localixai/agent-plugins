import { parseFrontmatter } from './frontmatter.ts';
import { readFlag, readText, type FieldRead } from './frontmatter-values.ts';
import { warning, type PluginIssue } from './issues.ts';

/**
 * A custom output style (output-styles.md, "Create a custom output style"): a Markdown file whose
 * frontmatter is metadata and whose body is the instructions Claude Code sends with every request.
 */
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

/** `output-styles/terse.md` is `terse`; both `/` and `\` separate a path. */
function fileName(path: string): string {
  const base = path.split(/[\\/]/).pop() ?? path;
  return base.replace(/\.md$/i, '');
}

/**
 * A `name` that is not a string is read as its JavaScript string conversion (spec J6.2, oracle
 * 2.1.319 and 2.1.322): `[a, b]` is `a,b`, a map `[object Object]`; a blank one is absent.
 */
function styleName(
  value: unknown,
  take: (read: FieldRead<string>) => string | undefined,
): string | undefined {
  if (typeof value !== 'object' || value === null) return take(readText(FIELD_KEYS.name, value));
  // JavaScript's conversion, written out: a list joins its items with `,`, a map is `[object Object]`.
  const text = (Array.isArray(value) ? value.join(',') : '[object Object]').trim();
  return text === '' ? undefined : text;
}

const FIELD_KEYS = {
  name: 'name',
  description: 'description',
  keep: 'keep-coding-instructions',
  force: 'force-for-plugin',
} as const;

/**
 * Read one output style file. Never throws and never refuses a file (output-styles.md: "If the
 * YAML doesn't parse, the style still loads under its file name with no fields set"; "A misspelled
 * field is ignored without an error"). What the docs leave open is in spec/judgments.md: a YAML
 * error and a value of the wrong shape are warnings, never errors.
 */
export function readOutputStyle(
  path: string,
  text: string,
  context: OutputStyleContext = {},
): OutputStyleRead {
  const parsed = parseFrontmatter(text);
  const issues: PluginIssue[] = [];
  if (parsed.problem !== undefined) {
    const where = parsed.line === undefined ? '' : ` (line ${parsed.line})`;
    issues.push(
      warning(
        path,
        `frontmatter ${parsed.problem}${where}; the style loads under its file name with no fields set`,
      ),
    );
  }
  const take = <T>(read: FieldRead<T>): T | undefined => {
    if (read.problem) issues.push(warning(path, `${read.problem.key} ${read.problem.reason}`));
    return read.value;
  };
  const { fields } = parsed;
  const name = styleName(fields[FIELD_KEYS.name], take) ?? fileName(path);
  const description = take(readText(FIELD_KEYS.description, fields[FIELD_KEYS.description]));
  const keep = take(readFlag(FIELD_KEYS.keep, fields[FIELD_KEYS.keep])) ?? false;
  const force = take(readFlag(FIELD_KEYS.force, fields[FIELD_KEYS.force])) ?? false;
  const { plugin } = context;
  return {
    style: {
      name,
      id: plugin === undefined ? name : `${plugin}:${name}`,
      ...(description === undefined ? {} : { description }),
      keepCodingInstructions: keep,
      forceForPlugin: plugin !== undefined && force,
      instructions: parsed.body.trim(),
      path,
    },
    issues,
  };
}

/**
 * Of one plugin's styles that give the same `name`, exactly one is kept: the one whose path
 * relative to the plugin root sorts last by UTF-16 code unit (spec J6.7, oracle 2.1.325: of
 * `terse.md` and `terse2.md` both named `Terse`, `terse2.md` is listed). A fixed order keeps the
 * result independent of the file system's. The kept style stays where it was.
 */
export function lastPerName(styles: readonly OutputStyle[]): OutputStyle[] {
  const kept = new Map<string, OutputStyle>();
  for (const style of styles) {
    const held = kept.get(style.name);
    if (held === undefined || held.path < style.path) kept.set(style.name, style);
  }
  return styles.filter((style) => kept.get(style.name) === style);
}
