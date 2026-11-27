import { coreModule } from '../../../wasm/bridge.ts';
import type { PathContext } from '../../types.ts';

/**
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/inputs/tool_path.rs`,
 * docs/RUST.md).
 */
const call = coreModule('rules/tools/inputs/tool-path');

type At = Pick<PathContext, 'cwd' | 'home' | 'platform'>;

/**
 * The absolute path Claude Code hands a hook for a file tool (hooks.md, "PreToolUse input"): "expands
 * `~` and relative paths before hooks run", and "On Windows, the path arrives with backslash
 * separators". `.` and `..` fold, in an absolute path too, so no spelling hides the target; a UNC
 * path keeps its two leading backslashes (spec/judgments.md).
 */
export const expandToolPath = (path: string, at: At): string => call('expandToolPath', path, at);

/**
 * A copy of a tool call's input as a hook receives it: the `file_path` of `Write`, `Edit` and `Read`
 * expanded (`expandToolPath`), every other field and tool untouched. An input that is not an object
 * has no fields, and a `file_path` that is not text is left as it was.
 */
export const expandToolInputPath = (
  toolName: string,
  input: unknown,
  at: At,
): Record<string, unknown> => call('expandToolInputPath', toolName, input, at);
