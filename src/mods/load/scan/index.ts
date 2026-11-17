import { scanIssue, type ModScanIssue } from './issue-codes.ts';
import { MOD_LIMITS } from '../../../protocol/mods-limits.ts';
import { isProgram } from './ast.ts';
import { fileFacts } from './file-rules.ts';
import { parseModule } from '../../../format/js-parse.ts';
import { clientModules } from './client-modules.ts';
import { newRun, scanRegister } from './rules.ts';
import type { EngineFold } from './engine-fold.ts';
import { emptyFound, type Found } from './found.ts';
import { indexProgram, notDeclared } from './top-level.ts';
import { lineIndex } from './on-rules.ts';
import { missingRegister } from './register-export.ts';
import type { ModScan } from './uses.ts';
import { maxScopeDepth } from './walk.ts';

export { combineScans } from './uses.ts';
export type { ModScan, ModUses, ScannedCall, ScannedHook } from './uses.ts';
export type { SurfaceModuleSpan } from './client-modules.ts';
export { MOD_SCAN_ISSUE_CODES, type ModScanIssue, type ModScanIssueCode } from './issue-codes.ts';

export interface ScanOptions {
  /** `plugin.key` of every value the declaration file declares; absent when it is not read. */
  readonly declaredState?: ReadonlySet<string>;
  /** The mod's own plugin name: only its `$.state` values are checked against the declarations. */
  readonly plugin?: string;
  /** The module the manifest names: only it must export `register`. */
  readonly isEntry?: boolean;
  /** The folder of the file relative to the plugin root, for the rule that an import stays inside it; the folder of `file` when absent. */
  readonly directory?: string;
}

/** The folder part of a path written with `/`. */
const directoryOf = (path: string): string => path.slice(0, Math.max(path.lastIndexOf('/'), 0));

/** The `engine.create` hooks in the form, together: what they add and withhold, each once. */
const foldsOf = ({ folds }: Found): { engine?: EngineFold } =>
  folds.length === 0
    ? {}
    : {
        engine: {
          adds: [...new Set(folds.flatMap((fold) => fold.adds))],
          withholds: [...new Set(folds.flatMap((fold) => fold.withholds))],
        },
      };

const TOO_DEEP = `code nested too deep to scan: more than ${MOD_LIMITS.scopes} scopes`;

const nothing = (file: string): Omit<ModScan, 'issues'> => ({
  file,
  hasRegister: false,
  hooks: [],
  calls: [],
  envReads: [],
  envWrites: [],
  stateReads: [],
  stateWrites: [],
  imports: [],
  surfaceModules: [],
  surfaceModuleSpans: [],
});

/**
 * The `$.state` values the hooks read or write, as `$.state.get` and `$.state.set` refs, that the
 * declarations do not name: the module does not load (§10.3.2 item 3). Another plugin's value is
 * not checked, and one an atom's declaration already reported is not reported twice.
 */
function undeclaredUses(
  file: string,
  found: Found,
  options: ScanOptions,
  reported: readonly ModScanIssue[],
): ModScanIssue[] {
  const { declaredState: declared, plugin } = options;
  if (declared === undefined) return [];
  const said = new Set(reported.map((issue) => issue.message));
  return [...new Set([...found.stateReads, ...found.stateWrites])].flatMap((value) => {
    const dot = value.indexOf('.');
    const ref = { plugin: value.slice(0, dot), key: value.slice(dot + 1) };
    const isOwn = plugin === undefined || plugin === ref.plugin;
    if (dot < 1 || !isOwn || declared.has(value)) return [];
    const issue = scanIssue('state-undeclared', file, notDeclared(ref));
    return said.has(issue.message) ? [] : [issue];
  });
}

/**
 * The static scan of one file of a hooks module (plugins/mods/create.md, "Check what Claude Code
 * reads from your mod"): the hooks it registers with their filters, the mods API calls it makes,
 * the environment variables and `$.state` values it reads and writes, and every way it breaks the
 * rules the scan relies on. It runs no code and never throws.
 */
export function scanHooksModule(source: string, file: string, options: ScanOptions = {}): ModScan {
  const parsed = parseModule(source, file);
  const { program } = parsed;
  if (!program || !isProgram(program))
    return {
      ...nothing(file),
      issues: parsed.issues.map((issue) => ({ ...issue, code: 'syntax' })),
    };
  if (maxScopeDepth(program) > MOD_LIMITS.scopes) {
    return { ...nothing(file), issues: [scanIssue('too-deep', file, TOO_DEEP)] };
  }
  const directory = options.directory ?? directoryOf(file);
  const facts = fileFacts(program, file, directory);
  const clients = clientModules(program, file, directory);
  const lineOf = lineIndex(source);
  const where = { file, declared: options.declaredState, plugin: options.plugin, lineOf };
  const top = indexProgram(program, where);
  const found = emptyFound();
  const run = newRun(file, top, found, lineOf);
  if (top.register) scanRegister(run, top.register);
  const issues = [...facts.issues, ...clients.issues, ...top.issues, ...found.issues];
  issues.push(...undeclaredUses(file, found, options, issues));
  if (options.isEntry === true && !top.register) {
    issues.push(scanIssue('no-register', file, missingRegister(program)));
  }
  return {
    file,
    hasRegister: top.register !== undefined,
    hooks: found.hooks,
    calls: [...found.calls.values()],
    envReads: [...found.envReads],
    envWrites: [...found.envWrites],
    stateReads: [...found.stateReads],
    stateWrites: [...found.stateWrites],
    imports: facts.imports,
    surfaceModules: clients.modules,
    surfaceModuleSpans: clients.spans,
    issues,
    ...foldsOf(found),
  };
}
