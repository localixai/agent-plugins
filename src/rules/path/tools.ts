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

/** The absolute POSIX path `call` acts on (a search without a path acts on the current directory). */
export const requestedPath = (toolCall: ToolCall, paths: PathContext): string | undefined =>
  call('requestedPath', toolCall, paths);
