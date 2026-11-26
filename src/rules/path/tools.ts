import type { PermissionBehavior } from '../../protocol/official.ts';
import type {
  FileEditInput,
  FileReadInput,
  FileWriteInput,
  GlobInput,
  GrepInput,
  NotebookEditInput,
} from '../../protocol/official-tools.ts';
import type { Assert, Equal } from '../../protocol/type-equality.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { PathContext, ToolCall } from '../types.ts';

const call = coreModule('rules/path/tools');

/**
 * The path tools and the field each carries its path in: `file_path` for Read, Edit and Write, `path`
 * for Grep and Glob, `notebook_path` for NotebookEdit (permissions.md, "Match by input parameter";
 * hooks.md, the PreToolUse input tables). A search tool's path is a directory and, when omitted, "the
 * current working directory will be used" (the SDK's own field description).
 */
export interface PathTool {
  readonly field: string;
  readonly searches: boolean;
  /** "file-reading tools, such as Read, Grep, and Glob" (permissions.md, "Network paths"). */
  readonly reads: boolean;
  /** The field of a pattern that, written as an absolute or `~` path, says where the search is. */
  readonly pattern?: string;
}

/** @public A compile-time drift guard: the SDK's path fields are strings (Glob and Grep optional). */
export type PathFieldsAreText = Assert<
  Equal<
    [
      FileReadInput['file_path'],
      FileEditInput['file_path'],
      FileWriteInput['file_path'],
      NotebookEditInput['notebook_path'],
      NonNullable<GlobInput['path']>,
      NonNullable<GrepInput['path']>,
    ],
    [string, string, string, string, string, string]
  >
>;

export const PATH_TOOLS: ReadonlyMap<string, PathTool> = call('PATH_TOOLS');

/**
 * The text of the path a call acts on, as sent (a rooted Glob pattern's directory before the `path`
 * field); `undefined` for a tool without one or a non-string.
 */
export const pathText = (toolCall: ToolCall): string | undefined => call('pathText', toolCall);

/** The absolute POSIX path `call` acts on (a search without a path acts on the current directory). */
export const requestedPath = (toolCall: ToolCall, paths: PathContext): string | undefined =>
  call('requestedPath', toolCall, paths);

/** Whether the tool's path is a directory searched, so a pattern is tried against what is below it. */
export const searchesDirectory = (toolName: string): boolean => call('searchesDirectory', toolName);

/**
 * Which call tools a path rule of `ruleTool` and `behavior` covers (the tool itself is always one):
 * "`Edit` rules apply to all built-in tools that edit files" (Write, NotebookEdit, the legacy
 * MultiEdit), Read rules reach Grep and Glob, a Read deny or ask rule also LSP ("Read, Grep, Glob,
 * LSP", tools-reference.md; matched against its `filePath`, J7.210), "An `Edit(...)` allow rule also grants read access to the same path", and "A
 * `Read(...)` deny rule also blocks the Edit and Write tools" but not NotebookEdit (permissions.md,
 * "What `Read` and `Edit` rules cover"; tools-reference.md).
 */
type Coverage = { readonly [B in PermissionBehavior]: readonly string[] };

/** The call tools whose path field may be absent: a rule that reaches such a call covers it (J7.210). */
export const PATHLESS_CALLS: ReadonlySet<string> = call('PATHLESS_CALLS');

export const COVERAGE: ReadonlyMap<string, Coverage> = call('COVERAGE');
