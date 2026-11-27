import type { CommandResult } from '../../../ports/index.ts';

/**
 * Background command hooks (hooks.md, "Run hooks in the background" and "Command hook fields").
 * `async: true` "runs in the background without blocking"; `asyncRewake: true` "runs in the
 * background and wakes Claude on exit code 2", and "implies async" (the SDK type). The timeout is
 * not enforced on an `async` hook but is on an `asyncRewake` one: that is decided where the
 * limit is computed (`handlerTimeoutMs`), so the request here already carries it or not.
 *
 * The runs themselves are the Rust core's (`crates/agent-plugins/src/hooks/handlers/command/async.rs`,
 * docs/RUST.md); a runner's answer type stays here.
 */

/**
 * What a runner answers when it can tell that a process the hook started still holds the output
 * open: "Claude Code finishes reading the hook's output shortly after the hook's own process
 * exits", and such a capture is suspect (docs/API.md, "Ports"). The `CommandRunner` port does not
 * type the field (`src/ports` is not this stream's), so a runner declares its answer as this type
 * and the compiler checks the name.
 */
export interface HeldOpenResult extends CommandResult {
  readonly outputHeldOpen?: boolean;
}
