/**
 * Themes and workflows (plugins/components.md, "Themes and output styles"; workflows.md, "Distribute
 * a workflow in a plugin"): each file is read once, the theme through `readTheme` and the workflow's
 * `meta` through `inspectWorkflowScript`, which never runs the script. What is wrong with a file comes
 * back as issues at the file's path; the component stays in the plugin either way, so a host sees the
 * file and the reason (a theme that is not a JSON object has no `theme`; a script without a readable
 * `meta` has no `meta` and no qualified name).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/readers_assets.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { PluginTheme, PluginWorkflow } from './plugin-model.ts';
import type { ComponentRead, ReaderContext } from './reader-types.ts';

const call = coreModule('loading/readers-assets');

export const readThemes = (
  context: ReaderContext,
): Promise<ComponentRead<readonly PluginTheme[]>> => call.async('readThemes', context);

export const readWorkflows = (
  context: ReaderContext,
): Promise<ComponentRead<readonly PluginWorkflow[]>> => call.async('readWorkflows', context);
