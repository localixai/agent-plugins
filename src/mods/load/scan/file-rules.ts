import type { Node } from 'acorn';
import { scanIssue, type ModScanIssue } from './issue-codes.ts';
import { isCall, isIdentifier, isImportExpression, stringOf } from './ast.ts';
import { childrenOf } from './walk.ts';
import { isHooksModuleName } from '../../../format/mod-files.ts';

/**
 * The rules that hold for a whole file of a hooks module (plugins/mods/create.md, "Check what Claude
 * Code reads from your mod"): imports are relative or `claude-code`, a file imported is named like
 * code, and a dynamic `import()` and `require` are refused. The words of the import, dynamic import
 * and file name rules are Claude Code 2.1.325's (Agent Plugins Extended §10.3.1 item 4, §10.4.1,
 * Judgment J10.2).
 */

const BAD_IMPORT =
  'a hooks module imports its own files by relative path and "claude-code", nothing else';
const OUTSIDE =
  'a hooks module imports only files inside the plugin directory, by relative path, and claude-code';
const DYNAMIC_IMPORT =
  'a dynamic import(); a hooks module imports its own files with an import declaration, as in import { name } from "./file.js"';
const NOT_CODE =
  'is not named like code and was not loaded: a hooks module and the files it imports end in .ts, .tsx, .jsx, .js, .mjs, .cjs, .mts or .cts';
const REQUIRE_CALL = 'require() is not available; write every file as an ES module with import';

export const isRelative = (specifier: string): boolean =>
  specifier.startsWith('./') || specifier.startsWith('../');

/** Whether a relative `specifier`, written in a file of `directory` (relative to the plugin root), names a file outside the plugin directory. */
export function leavesRoot(directory: string, specifier: string): boolean {
  let depth = directory.split('/').filter((part) => part !== '' && part !== '.').length;
  for (const part of specifier.split('/')) {
    if (part === '..') depth -= 1;
    else if (part !== '.' && part !== '') depth += 1;
    if (depth < 0) return true;
  }
  return false;
}

/** The path from the plugin root of `specifier`, written in a file of `directory`. */
function joined(directory: string, specifier: string): string {
  const parts: string[] = [];
  for (const part of [...directory.split('/'), ...specifier.split('/')]) {
    if (part === '..') parts.pop();
    else if (part !== '.' && part !== '') parts.push(part);
  }
  return parts.join('/');
}

/** The file's path from the plugin root: its name in its folder `directory`. */
const fromRoot = (file: string, directory: string): string =>
  joined(directory, file.split('/').at(-1) ?? file);

/** The issue of a relative import, or `undefined` when the file it names may be loaded. */
function relativeProblem(file: string, directory: string, from: string): ModScanIssue | undefined {
  if (leavesRoot(directory, from)) {
    return scanIssue(
      'import-outside',
      file,
      `imports "${from}", outside the plugin directory: ${OUTSIDE}`,
    );
  }
  // An import without a suffix is the walker's to resolve; one with another suffix is not code.
  const name = from.split('/').at(-1) ?? '';
  if (!name.includes('.') || isHooksModuleName(from)) return undefined;
  return scanIssue(
    'import-not-code',
    file,
    `cannot import "${from}" (from ${fromRoot(file, directory)}): ${joined(directory, from)} ${NOT_CODE}`,
  );
}

const FROM_TYPES: ReadonlySet<string> = new Set([
  'ImportDeclaration',
  'ExportNamedDeclaration',
  'ExportAllDeclaration',
]);

/** The module an import or re-export names, as a literal. */
function fromOf(node: Node): string | undefined {
  if (!FROM_TYPES.has(node.type)) return undefined;
  const source: unknown = Reflect.get(node, 'source');
  return typeof source === 'object' && source !== null && 'type' in source && 'start' in source
    ? // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- a `source` with `type` and `start` is a node
      stringOf(source as Node)
    : undefined;
}

export interface FileFacts {
  /** Relative import specifiers, for the walker to follow. */
  readonly imports: readonly string[];
  readonly issues: readonly ModScanIssue[];
}

/**
 * Imports, dynamic imports and `require` calls anywhere in the file. `directory` is the folder of the
 * file, relative to the plugin root: "Import only from files inside the plugin directory" (create.md).
 */
export function fileFacts(program: Node, file: string, directory: string): FileFacts {
  const imports: string[] = [];
  const issues: ModScanIssue[] = [];
  const stack: Node[] = [program];
  for (let node = stack.pop(); node; node = stack.pop()) {
    stack.push(...childrenOf(node).toReversed());
    if (isImportExpression(node)) issues.push(scanIssue('dynamic-import', file, DYNAMIC_IMPORT));
    if (isCall(node) && isIdentifier(node.callee) && node.callee.name === 'require') {
      issues.push(scanIssue('require', file, REQUIRE_CALL));
    }
    const from = fromOf(node);
    if (from === undefined) continue;
    if (isRelative(from)) {
      const problem = relativeProblem(file, directory, from);
      if (problem === undefined) imports.push(from);
      else issues.push(problem);
    } else if (from !== 'claude-code') {
      const at = fromRoot(file, directory);
      issues.push(
        scanIssue('import-package', file, `cannot import "${from}" (from ${at}): ${BAD_IMPORT}`),
      );
    }
  }
  return { imports, issues };
}
