/**
 * `--permission-mode` and `--tools` (cli-reference.md, "CLI flags"). `--permission-mode` "accepts
 * `default`, `acceptEdits`, `plan`, `auto`, `dontAsk`, `bypassPermissions`, or `manual` as an alias
 * for `default`" and "overrides `defaultMode` from settings files"; a `--restricted` session "refuses
 * `bypassPermissions`". `--tools` "restricts which built-in tools Claude can use": `""` disables
 * all, `"default"` is the default set, or tool names like `"Bash,Edit,Read"`; naming a task-tracking
 * tool "also opts the session in". The wording of a refusal is in spec/judgments.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/flags/session.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { Contribution, FlagContext } from './types.ts';

const call = coreModule('cli/flags/session');

export const permissionMode = (value: string, context: FlagContext): Contribution =>
  call('permissionMode', value, context);

export const tools = (value: string): Contribution => call('tools', value);
