/**
 * A shell-form hook `command`, which runs through a shell (hooks.md, "Exec form and shell form";
 * errors.md, "Plugin command references user_config in a shell command"). The message is the one
 * errors.md quotes for a hook: `Hook from plugin <plugin> references ${user_config.*} in a
 * shell-form command`, the way out, and the offending template after `Command:`.
 *
 * Judgment: `<plugin>` is the plugin identifier the host uses (`name@marketplace`, or the bare
 * name), the form errors.md prints (`formatter@acme-tools`); `<command>` is the template as written.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/shell_guard_hook.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { ShellGuard } from './shell-guard.ts';

const call = coreModule('loading/resolve/shell-guard-hook');

export const HOOK_GUARD: ShellGuard<{ readonly plugin: string; readonly command: string }> =
  call('HOOK_GUARD');
