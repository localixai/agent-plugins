/**
 * The plugin workflows a session offers as commands.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/session_workflows.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import { portsOf } from './session-environment.ts';
import type { SessionOptions } from './session-types.ts';
import type { HostState } from './state.ts';

const call = coreModule('host/session-workflows');

/**
 * A plugin workflow the session offers as `/<plugin>:<meta.name>` (workflows.md, "Distribute a
 * workflow in a plugin"). The kit lists it; the host runs the script at `path` (below the plugin's
 * root) with its Workflow tool.
 */
export interface WorkflowCommand {
  readonly name: string;
  readonly description: string;
  readonly pluginId: string;
  readonly path: string;
}

/**
 * The workflow commands of the plugins that run, in their loading order. Oracle 2.1.319
 * (spec/judgments.md): a script whose `meta` does not read (not the first statement, no description)
 * is not one, and of two scripts with one `meta.name` the first in the folder listing is kept. The
 * state is the host's object: its fields the core reads cross, the view and the environment as ports.
 */
export const workflowCommandsOf = (
  state: HostState,
  options: SessionOptions,
): readonly WorkflowCommand[] =>
  call(
    'workflowCommandsOf',
    state.active.map((plugin) => ({ id: plugin.id, workflows: plugin.plugin.workflows })),
    state.view,
    state.options.workflows,
    portsOf(state, options).environment,
  );
