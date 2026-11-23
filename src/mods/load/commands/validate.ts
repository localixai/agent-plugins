import { error, type PluginIssue } from '../../../format/issues.ts';
import {
  readHooksJsonModules,
  resolveTypesPath,
  isHooksModuleName,
} from '../../../format/mod-files.ts';
import { validateManifest } from '../../../format/manifest.ts';
import { scanSourceOf, type CompileResult } from '../compile-output.ts';
import { thrownByCompiler } from '../engine-faults.ts';
import { declaredState, missingTypes, readTypes } from '../types-decl.ts';
import { scanHooksModule, type ModScan } from '../scan/index.ts';
import { ledEntry, ledRule } from './scan-lead.ts';
import { checkWasmFiles, wasmLines } from '../wasm-files.ts';
import { gatingHooksOf, moduleFacts } from './module-facts.ts';
import { scanLines, typesNotes } from './scan-lines.ts';
import { readJson } from '../../../format/read-json.ts';
import { isRecord } from '../../../protocol/is-record.ts';
import type { FileReport, ReportEntry, ValidateInput, ValidateReport } from './validate-report.ts';

/**
 * `claude plugin validate <directory>` for a mod (plugins/mods/reference.md, "Commands"; create.md,
 * "Check what Claude Code reads from your mod"; plugins/cli-reference.md, "plugin validate"): it reads
 * the manifest and the hooks module and reports errors, the events the module handles and the mods
 * API calls it makes. `--strict` treats warnings as errors and `--json` prints a machine-readable
 * report. It runs no code and never throws: what it cannot read is an issue, and the exit code says so.
 */

const JSON_PATH = '.claude-plugin/plugin.json';
const HOOKS_PATH = 'hooks/hooks.json';

/**
 * A `hooks.json` with neither `hooks` nor `modules` fails at `root` (oracle 2.1.321 and 2.1.325;
 * Agent Plugins Extended §10.42.3 item 1, Judgment J10.174): the docs say it passes.
 */
const NEITHER_KEY =
  'hooks.json must have `hooks` (the hook matchers) or `modules` (hooks modules), or both';
const neitherKey = (value: unknown): PluginIssue[] =>
  isRecord(value) && !('hooks' in value) && !('modules' in value)
    ? [error('root', NEITHER_KEY)]
    : [];

const messageOf = (issue: PluginIssue): ReportEntry => ({
  path: issue.path,
  message: issue.message,
});
/** An entry as the command prints it: `<path>: <message>`. */
const printed = (entry: ReportEntry): string => `${entry.path}: ${entry.message}`;
const errorsOf = (issues: readonly PluginIssue[]) =>
  issues.filter((issue) => issue.severity === 'error').map(messageOf);
const warningsOf = (issues: readonly PluginIssue[]) =>
  issues.filter((issue) => issue.severity === 'warning').map(messageOf);

/**
 * What `validate` reads of a compiler result: the scan output (a compiler with only a scan output is
 * enough, `validate` runs nothing). As on load, the scan's own problem is the reason when the
 * compiler gave an output, and the compiler's first issue only when it gave none.
 */
function scanOf(
  compiled: CompileResult,
  entry: string,
): { readonly code: string } | { readonly issue: PluginIssue } {
  const source = scanSourceOf(compiled, entry);
  if ('code' in source) return source;
  const hasOutput = compiled.scan !== undefined || compiled.run !== undefined;
  return { issue: (hasOutput ? undefined : compiled.issues[0]) ?? error(entry, source.problem) };
}

/** The module's source as the scan reads it: written JavaScript as it is, anything else through the compiler. */
async function sourceOf(
  input: ValidateInput,
  entry: string,
): Promise<{ readonly code: string } | { readonly issue: PluginIssue }> {
  const { files, compiler, signal } = input;
  const own = entry.endsWith('.js') || entry.endsWith('.mjs') || entry.endsWith('.cjs');
  if (own) {
    const text = await files.readText(entry, signal);
    return text === undefined
      ? { issue: error(entry, `${entry}: the hooks module does not exist`) }
      : { code: text };
  }
  if (!compiler) {
    return {
      issue: error(entry, `${entry}: a TypeScript or JSX hooks module needs a compiler to be read`),
    };
  }
  try {
    const compiled = await compiler.compile(
      { files, entry },
      signal ?? new AbortController().signal,
    );
    return scanOf(compiled, entry);
  } catch (thrown) {
    return { issue: thrownByCompiler(entry, thrown) };
  }
}

interface Module {
  readonly report: FileReport;
  /** The lines the command prints for the module, `  ❯ ./register.js hooks: ...`. */
  readonly lines: readonly string[];
}

/** oracle 2.1.322: a surface module a `Client` names that is not there is an error of the module. */
async function missingSurfaces(
  input: ValidateInput,
  written: string,
  scan: ModScan,
): Promise<PluginIssue[]> {
  const present = await Promise.all(
    scan.surfaceModules.map((path) => input.files.exists(path, input.signal)),
  );
  return scan.surfaceModules
    .filter((_, index) => present[index] !== true)
    .map((path) => error(written, `${path}: no such file`));
}

/** What the manifest says the module is checked against: its types file and its plugin's name. */
interface Contract {
  readonly typesPath: string | undefined;
  readonly plugin: string | undefined;
}

async function checkModule(
  input: ValidateInput,
  entry: string,
  written: string,
  { typesPath, plugin }: Contract,
): Promise<Module> {
  const source = await sourceOf(input, entry);
  if ('issue' in source) {
    return {
      report: {
        file: written,
        errors: [messageOf(source.issue)],
        warnings: [],
        notes: [],
        gatingHooks: [],
        modules: null,
      },
      lines: [],
    };
  }
  const types = await readTypes(input.files, typesPath, input.signal);
  const scan = scanHooksModule(source.code, written, {
    isEntry: true,
    ...(plugin === undefined ? {} : { plugin }),
    directory: entry.slice(0, Math.max(entry.lastIndexOf('/'), 0)),
    ...(types === undefined ? {} : { declaredState: declaredState(types) }),
  });
  const issues = [...scan.issues, ...(await missingSurfaces(input, written, scan))];
  const needed = types === undefined ? missingTypes(scan, written, typesPath) : undefined;
  if (needed !== undefined) issues.push(error(typesPath ?? JSON_PATH, needed));
  const place = { written, entry, root: input.root ?? input.target };
  const wasm = await checkWasmFiles(input.files, scan.wasmFiles, {
    ...place,
    signal: input.signal,
  });
  const lines = [...scanLines(written, scan, plugin), ...wasmLines(written, wasm.files)];
  const led = (severity: 'error' | 'warning') =>
    wasm.problems
      .filter((problem) => problem.severity === severity)
      .map((problem) => ledRule(place, problem.line, problem.message));
  return {
    lines,
    report: {
      file: written,
      errors: [
        ...issues
          .filter((issue) => issue.severity === 'error')
          .map((issue) => ledEntry(issue, place)),
        ...led('error'),
      ],
      warnings: led('warning'),
      notes: lines.map((line) => line.replace(/^ {2}❯ /, '')),
      gatingHooks: gatingHooksOf(written, scan),
      wasm: wasm.files,
      modules: [moduleFacts(entry, scan)],
    },
  };
}

function verdict(counts: {
  readonly errors: number;
  readonly warnings: number;
  readonly strict: boolean;
}): { readonly line: string; readonly exitCode: 0 | 1 } {
  if (counts.errors > 0) return { line: '✘ Validation failed', exitCode: 1 };
  if (counts.warnings > 0 && counts.strict) {
    return { line: '✘ Validation failed (--strict treats warnings as errors)', exitCode: 1 };
  }
  const line = counts.warnings > 0 ? '✔ Validation passed with warnings' : '✔ Validation passed';
  return { line, exitCode: 0 };
}

/** The manifest and its `types` path, read from `.claude-plugin/plugin.json`. */
async function checkManifest(
  input: ValidateInput,
): Promise<{ readonly report: FileReport } & Contract> {
  const read = await readJson(input.files, JSON_PATH, input.signal);
  const issues: PluginIssue[] =
    'value' in read
      ? validateManifest(read.value)
      : [
          error(
            JSON_PATH,
            `${JSON_PATH}: ${read.missing === true ? 'there is no manifest' : 'the manifest is not JSON'}`,
          ),
        ];
  const declared =
    'value' in read && typeof read.value === 'object' && read.value !== null
      ? Reflect.get(read.value, 'types')
      : undefined;
  const name: unknown = 'value' in read && isRecord(read.value) ? read.value.name : undefined;
  const typesPath = resolveTypesPath({
    types: typeof declared === 'string' ? declared : undefined,
  });
  const notes = await typesNotes(input.files, typesPath, input.signal);
  return {
    report: {
      file: JSON_PATH,
      errors: errorsOf(issues),
      warnings: warningsOf(issues),
      notes,
      gatingHooks: [],
    },
    typesPath,
    plugin: typeof name === 'string' ? name : undefined,
  };
}

/** The files of the hooks module: what `hooks/hooks.json` says, and the module it names. */
async function checkModules(
  input: ValidateInput,
  contract: Contract,
): Promise<{ readonly files: readonly FileReport[]; readonly lines: readonly string[] }> {
  const read = await readJson(input.files, HOOKS_PATH, input.signal);
  const modules = 'value' in read ? readHooksJsonModules(read.value) : { issues: [] };
  const issues = 'value' in read ? [...neitherKey(read.value), ...modules.issues] : modules.issues;
  const hooksFile: FileReport[] =
    issues.length === 0
      ? []
      : [
          {
            file: HOOKS_PATH,
            errors: errorsOf(issues),
            warnings: warningsOf(issues),
            notes: [],
            gatingHooks: [],
          },
        ];
  const { entry, written } = modules;
  if (entry === undefined || written === undefined || !isHooksModuleName(entry)) {
    return { files: hooksFile, lines: [] };
  }
  const module = await checkModule(input, entry, written, contract);
  return { files: [...hooksFile, module.report], lines: module.lines };
}

/** "Files with no error, warning, or note get no entry, so `contents` is empty when there's nothing to report" (cli-reference.md, "Output and exit codes"). */
const hasEntry = (file: FileReport): boolean =>
  file.errors.length + file.warnings.length + file.notes.length > 0;

const sum = (files: readonly FileReport[], key: 'errors' | 'warnings'): number =>
  files.reduce((total, file) => total + file[key].length, 0);

export async function validateMod(
  input: ValidateInput,
  options: { readonly strict?: boolean } = {},
): Promise<ValidateReport> {
  const strict = options.strict === true;
  const { report: manifest, ...contract } = await checkManifest(input);
  const modules = await checkModules(input, contract);
  const all = [manifest, ...modules.files];
  const done = verdict({ errors: sum(all, 'errors'), warnings: sum(all, 'warnings'), strict });
  const lines = [
    `Validating plugin manifest: ${input.target}/${JSON_PATH}`,
    ...all.flatMap((file) => file.errors.map((entry) => `  ✘ ${printed(entry)}`)),
    ...all.flatMap((file) => file.warnings.map((entry) => `  ⚠ ${printed(entry)}`)),
    ...manifest.notes.map((line) => `  ❯ ${line}`),
    ...modules.lines,
    done.line,
  ];
  return {
    success: done.exitCode === 0,
    strict,
    target: input.target,
    manifest,
    contents: modules.files.filter(hasEntry),
    advice: [],
    lines,
    exitCode: done.exitCode,
  };
}
