/**
 * The words the docs quote for a hook's errors (hooks.md, "Exit code 2", "Other exit codes",
 * "Block the action when a hook fails"), kept in one file so each is checked against its source.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/verdict/notices.rs`, docs/RUST.md).
 */
import type { HookHandler } from '../../protocol/hook-config.ts';
import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/verdict/notices');

/** `PreToolUse:Bash`: the event, and the tool when the event's input has one. */
export const hookName = (event: HookEvent, toolName: string | undefined): string =>
  call('hookName', event, toolName);

/**
 * The part of a handler the docs put in brackets: a command hook's command and arguments ("[node
 * ${CLAUDE_PROJECT_DIR}/.claude/hooks/check-command.js]"). Judgment (spec/judgments.md): the other
 * types show their URL, `server:tool`, or their type.
 */
export const handlerLabel = (handler: HookHandler): string => call('handlerLabel', handler);

/** "PreToolUse:Bash hook error: [${CLAUDE_PROJECT_DIR}/.claude/hooks/no-rm.sh]: Blocked: rm commands are not allowed". */
export const hookError = (name: string, label: string, text: string): string =>
  call('hookError', name, label, text);

/** "Failed with non-blocking status code: something broke". */
export const NON_BLOCKING_STATUS: string = call('NON_BLOCKING_STATUS');

/** Judgment (spec/judgments.md): the docs never show the notice of a hook that wrote nothing to stderr. */
export const NO_STDERR: string = call('NO_STDERR');

/** "the error includes `failed; blocking because onFailure is "block"` followed by node's own error output". */
export const failedBlocking = (detail: string): string => call('failedBlocking', detail);

/** "After a timeout, the message says `timed out` instead of `failed`." */
export const timedOutBlocking = (detail: string): string => call('timedOutBlocking', detail);

/**
 * "If the error for a blocked tool call includes this text, a `PreToolUse` or `PermissionRequest`
 * command hook failed or timed out and blocked the call".
 */
export const windowsBlocking = (detail: string): string => call('windowsBlocking', detail);

export const TIMED_OUT: string = call('TIMED_OUT');

/**
 * `PreModelSwitch`: a hook that doesn't respond before its timeout blocks the switch, with the
 * reason `PreModelSwitch hook <name> did not respond before its timeout` (errors.md, "Model switch
 * was blocked by a PreModelSwitch hook"; Agent Plugins Extended §7.5.33). The hook is named by its
 * handler label, as the hook error line names it.
 */
export const timedOutBlocksSwitch = (name: string): string => call('timedOutBlocksSwitch', name);

/**
 * "The transcript shows a notice naming the hook, the timeout that fired, and that the output was
 * discarded" (`UserPromptSubmit`). Judgment (spec/judgments.md): the docs give the notice no
 * wording; the hook is named by the line around it, the timeout is written in seconds.
 */
export const timedOutDiscarded = (timeoutMs: number | undefined): string =>
  call('timedOutDiscarded', timeoutMs);

export const firstLine = (text: string): string => call('firstLine', text);

/**
 * "When a plugin's hook blocks this way and its stderr is the blocking message, the error ends with
 * `This hook comes from the <plugin> plugin.` so that you know which plugin to disable or fix"
 * (plugins/troubleshooting.md). Judgment (spec/judgments.md): the docs give no separator, a space.
 */
export const fromPlugin = (message: string, plugin: string | undefined): string =>
  call('fromPlugin', message, plugin);
