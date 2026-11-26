import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';

/**
 * The files of a mod (plugins/mods/reference.md, "Files"): `hooks/hooks.json` names one hooks
 * module in `modules`, the module has one of eight extensions, `*.test.ts` and `*.test.tsx` are
 * tests, and `types` in the manifest may name the declaration file. Pure readers: a problem is a
 * `PluginIssue`, never a throw.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mod_files.rs`, docs/RUST.md).
 */
const call = coreModule('format/mod-files');

export const isHooksModuleName = (path: string): boolean => call('isHooksModuleName', path);

/** Tests `claude plugin test` runs: "files whose names end in `.test.ts` or `.test.tsx`". */
export const isModTestFile = (path: string): boolean => call('isModTestFile', path);

/**
 * The declaration file of a mod: the manifest's `types` path, else `types/index.d.ts`; relative to
 * the plugin folder, `/`-separated. `undefined` when the path leaves the plugin as written (`..`,
 * absolute, a backslash): there is no declaration file to read then (judgment, spec/judgments.md).
 */
export const resolveTypesPath = (manifest: {
  readonly types?: string | undefined;
}): string | undefined => call('resolveTypesPath', manifest);

export interface HooksModules {
  /** The one module path, relative to the plugin folder (`hooks/` joined to the entry), or `undefined` when the file names none. */
  readonly entry?: string;
  /** The path as written in `hooks.json`, relative to that file. */
  readonly written?: string;
  readonly issues: readonly PluginIssue[];
}

/**
 * Reads `modules` of `hooks/hooks.json`: "an array with one path, relative to this file" (`"modules":
 * ["./register.js"]`). A file with no `modules` key holds no mod (a mod-less plugin is valid), and
 * the entry is `undefined`; one with the wrong number of paths, or a path of the wrong kind, is an
 * error. Settings hooks under `hooks` are read by `readHooksFile`.
 */
export const readHooksJsonModules = (raw: unknown): HooksModules =>
  call('readHooksJsonModules', raw);
