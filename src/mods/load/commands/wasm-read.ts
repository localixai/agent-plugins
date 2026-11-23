/**
 * What the validator reads of a WebAssembly file a `$.wasm.compile` call names (Agent Plugins
 * Extended §10.4.2, §10.4.3; Claude Code 2.1.330): its imports and exports, or where it does not
 * parse, worded as `WebAssembly.Module` words it (`<path>: WebAssembly.Module doesn't parse at byte
 * <n>: <reason>`). Judgment (spec/judgments.md): the kit reads the header, each section's frame and
 * the import and export sections, not the rest of the module; the header and frame reasons are the
 * engine's words, those of a cut-short import or export the kit's in the same form.
 */

export interface WasmEntries {
  /** `<module>.<name> (<kind>)`, in the file's order. */
  readonly imports: readonly string[];
  /** `<name> (<kind>)`, in the file's order. */
  readonly exports: readonly string[];
}

export interface WasmParseFailure {
  readonly at: number;
  readonly reason: string;
}

const MAGIC = [0x00, 0x61, 0x73, 0x6d];
const SECTION_NAMES = [
  'Custom',
  'Type',
  'Import',
  'Function',
  'Table',
  'Memory',
  'Global',
  'Export',
  'Start',
  'Element',
  'Code',
  'Data',
  'DataCount',
  'Tag',
];
const KINDS = ['function', 'table', 'memory', 'global', 'tag'];
const IMPORT = 2;
const EXPORT = 7;

/** Thrown inside the reader only, and caught where it starts. */
class Stop extends Error {
  readonly at: number;
  readonly reason: string;
  constructor(at: number, reason: string) {
    super(reason);
    this.at = at;
    this.reason = reason;
  }
}

class Cursor {
  at = 0;
  readonly bytes: Uint8Array;
  readonly end: number;
  constructor(bytes: Uint8Array, end: number) {
    this.bytes = bytes;
    this.end = end;
  }

  byte(what: string): number {
    const value = this.at < this.end ? this.bytes[this.at] : undefined;
    if (value === undefined) throw new Stop(this.at, `can't get ${what}`);
    this.at += 1;
    return value;
  }

  /** An unsigned LEB128 of up to 32 bits. */
  u32(what: string): number {
    let value = 0;
    for (let shift = 0; shift < 35; shift += 7) {
      const byte = this.byte(what);
      value += (byte & 0x7f) * 2 ** shift;
      if ((byte & 0x80) === 0) return value;
    }
    throw new Stop(this.at, `can't get ${what}`);
  }

  name(what: string): string {
    const length = this.u32(`${what}'s length`);
    if (this.at + length > this.end)
      throw new Stop(this.at, `can't get ${what} of length ${length}`);
    const text = new TextDecoder().decode(this.bytes.subarray(this.at, this.at + length));
    this.at += length;
    return text;
  }
}

const limits = (cursor: Cursor, what: string): void => {
  const flags = cursor.byte(`${what}'s limits`);
  cursor.u32(`${what}'s minimum`);
  if ((flags & 1) === 1) cursor.u32(`${what}'s maximum`);
};

/** How each kind of import describes what it takes, read past (kind order as in `KINDS`). */
const DESCRIPTIONS: readonly ((cursor: Cursor, what: string) => void)[] = [
  (cursor, what) => void cursor.u32(`${what}'s signature`),
  (cursor, what) => (cursor.byte(`${what}'s element type`), limits(cursor, what)),
  (cursor, what) => limits(cursor, what),
  (cursor, what) => (cursor.byte(`${what}'s type`), cursor.byte(`${what}'s mutability`)),
  (cursor, what) => (cursor.byte(`${what}'s attribute`), cursor.u32(`${what}'s signature`)),
];

function kindOf(cursor: Cursor, what: string): string {
  const at = cursor.at;
  const kind = cursor.byte(`${what}'s kind`);
  const name = KINDS[kind];
  if (name === undefined) throw new Stop(at, `${what} has unknown kind ${kind}`);
  return name;
}

function readImports(cursor: Cursor): string[] {
  const count = cursor.u32("Import section's count");
  const out: string[] = [];
  for (let i = 0; i < count; i += 1) {
    const what = `${i}th Import`;
    const module = cursor.name(`${what}'s module name`);
    const field = cursor.name(`${what}'s field name`);
    const kind = kindOf(cursor, what);
    DESCRIPTIONS[KINDS.indexOf(kind)]?.(cursor, what);
    out.push(`${module}.${field} (${kind})`);
  }
  return out;
}

function readExports(cursor: Cursor): string[] {
  const count = cursor.u32("Export section's count");
  const out: string[] = [];
  for (let i = 0; i < count; i += 1) {
    const what = `${i}th Export`;
    const name = cursor.name(`${what}'s field name`);
    const kind = kindOf(cursor, what);
    cursor.u32(`${what}'s index`);
    out.push(`${name} (${kind})`);
  }
  return out;
}

function header(bytes: Uint8Array): void {
  if (bytes.length < 8) throw new Stop(0, 'expected a module of at least 8 bytes');
  if (MAGIC.some((byte, at) => bytes[at] !== byte)) {
    throw new Stop(0, "module doesn't start with '\\0asm'");
  }
  const version = new DataView(bytes.buffer, bytes.byteOffset, 8).getUint32(4, true);
  if (version !== 1) throw new Stop(0, `unexpected version number ${version} expected 1`);
}

function sections(bytes: Uint8Array): WasmEntries {
  const cursor = new Cursor(bytes, bytes.length);
  cursor.at = 8;
  let imports: string[] = [];
  let exports: string[] = [];
  while (cursor.at < bytes.length) {
    const idAt = cursor.at;
    const id = cursor.byte('section id');
    const name = SECTION_NAMES[id];
    if (name === undefined) throw new Stop(idAt, 'invalid section');
    const size = cursor.u32(`${name} section's size`);
    const start = cursor.at;
    if (start + size > bytes.length) {
      throw new Stop(start, `${name} section of size ${size} would overflow Module's size`);
    }
    const inner = new Cursor(bytes, start + size);
    inner.at = start;
    if (id === IMPORT) imports = readImports(inner);
    if (id === EXPORT) exports = readExports(inner);
    cursor.at = start + size;
  }
  return { imports, exports };
}

/** The imports and exports of a module, or where and why it does not parse. */
export function readWasm(bytes: Uint8Array): WasmEntries | WasmParseFailure {
  try {
    header(bytes);
    return sections(bytes);
  } catch (thrown) {
    if (thrown instanceof Stop) return { at: thrown.at, reason: thrown.reason };
    throw thrown;
  }
}
