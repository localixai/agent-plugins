/**
 * The writer of a Markdown file's frontmatter, the inverse of `parseFrontmatter`: what it writes
 * reads back to the same `fields` and the same `body` (tests/format/frontmatter-write.test.ts). The
 * YAML is written as the `yaml` package writes it, the parser's own, so a value that needs quoting
 * (`Use when: x`, `true` as text, `@file`) is quoted and never relies on the reader's relaxed
 * retry. A value is a JSON-shaped one (text, number, boolean, `null`, a list, a table); `undefined`
 * is "no key".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/frontmatter_write.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/frontmatter-write');

/**
 * A whole file: `---`, the fields in the order given (JavaScript lists integer-like keys first),
 * `---`, then `body` exactly as given, with no line added before it. An empty `fields` still writes
 * the two markers, so a body that opens with `---` stays body.
 */
export const writeFrontmatter = (fields: Readonly<Record<string, unknown>>, body: string): string =>
  call('writeFrontmatter', fields, body);

export type FrontmatterEdit =
  | { readonly ok: true; readonly text: string }
  | {
      readonly ok: false;
      /** The block does not read: writing would drop what the file holds, so nothing is written. */
      readonly state: 'invalid-yaml' | 'not-a-table';
      readonly problem: string;
    };

/**
 * `text` with `changes` set in its frontmatter: a key it has keeps its place, a new one goes after
 * the others in the order given, `undefined` removes one; every other key and the comments stay.
 * `body` replaces the body when given, else the body stays byte for byte. A file without a block
 * gets one, its whole text the body. A block that does not read is refused, not overwritten.
 */
export const editFrontmatter = (
  text: string,
  changes: Readonly<Record<string, unknown>>,
  body?: string,
): FrontmatterEdit => call('editFrontmatter', text, changes, body);
