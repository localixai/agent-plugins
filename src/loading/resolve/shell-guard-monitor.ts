/**
 * A monitor `command`, which runs through a shell (plugins/components.md, "Monitors"; errors.md,
 * "Plugin command references user_config in a shell command"). The message is the one errors.md
 * quotes for a monitor: `Monitor "<name>" from plugin <plugin> references ${user_config.*} in its
 * command` followed by the explanation.
 *
 * Judgment: `<plugin>` is the plugin identifier the host uses (`name@marketplace`, or the bare name),
 * the form the shell-form hook message of the same page shows (`formatter@acme-tools`); the monitor
 * example prints a name with no marketplace, which a bare identifier gives too.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/shell_guard_monitor.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { ShellGuard } from './shell-guard.ts';

const call = coreModule('loading/resolve/shell-guard-monitor');

export const MONITOR_GUARD: ShellGuard<{ readonly monitor: string; readonly plugin: string }> =
  call('MONITOR_GUARD');
