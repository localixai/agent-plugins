/**
 * A Markdown file's YAML frontmatter and body. The YAML is read as the `yaml` package reads it with
 * `{ prettyErrors: false, uniqueKeys: false, logLevel: 'error' }`, its errors worded and placed as
 * there; a block it refuses is retried with the values Claude Code quotes (oracle 2.1.319).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/frontmatter.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/frontmatter');

/**
 * Where the frontmatter stands:
 *
 * - `none`: the file does not open with a `---` line, or no closing `---` follows; all of it is
 *   body.
 * - `ok`: a YAML table (or an empty block) between the markers; `fields` holds it.
 * - `invalid-yaml`: the YAML between the markers does not parse; the file loads with no fields.
 * - `not-a-table`: it parses to a list or a single value; the file loads with no fields.
 */
export type FrontmatterState = 'none' | 'ok' | 'invalid-yaml' | 'not-a-table';

export interface Frontmatter {
  readonly state: FrontmatterState;
  /** `state` is not `none`: the file has a frontmatter block, valid or not. */
  readonly present: boolean;
  /** Always a plain object: empty unless `state` is `ok`. */
  readonly fields: Readonly<Record<string, unknown>>;
  /** The text after the closing marker, or the whole file when `state` is `none`. */
  readonly body: string;
  /** Why `state` is `invalid-yaml` or `not-a-table`, as the YAML parser or `readTable` words it, for a notice. */
  readonly problem?: string;
  /**
   * Where the problem is in the file, counted from 1 (the opening `---` is line 1): the first
   * YAML line is 2. Present when `problem` is.
   */
  readonly line?: number;
  readonly column?: number;
  /** What the block parsed to when `state` is `not-a-table` (a list or a single value). */
  readonly value?: unknown;
}

/** One line of `text` from `at`, and where the next line starts (past the end on the last line). */
export const lineAt = (text: string, at: number): { line: string; next: number } =>
  call('lineAt', text, at);

/**
 * Split a Markdown file into its YAML frontmatter and body, as skills.md and output-styles.md
 * describe: the opening `---` must be line 1, a block that does not parse leaves the file loaded
 * with no fields. Never throws; CRLF files read like LF ones.
 */
export const parseFrontmatter = (text: string): Frontmatter => call('parseFrontmatter', text);
