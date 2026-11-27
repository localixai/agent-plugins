import type { CommandRun } from '../ports/command-runner.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { NodeDepsInput, NodeDepsInstall, NodeDepsResult } from './node-deps-types.ts';

/**
 * The plan of a plugin's Node.js package install (plugins/loading.md, "Node.js package dependencies").
 * Pure: `NodeDepsInput` in, a plan out; the host copies the two files, runs the command through
 * `CommandRunner` and moves `node_modules` (docs/API.md, "Node.js package dependencies").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/node_deps.rs`, docs/RUST.md).
 */

const call = coreModule('marketplace/node-deps');

/** Whether and how Claude Code installs a plugin's Node.js packages. */
export const planNodeDepsInstall = (input: NodeDepsInput): NodeDepsResult =>
  call('planNodeDepsInstall', input);

/** The process the host runs for an install: the plan, the host's environment and abort signal; output merged so a failure shows its cause. */
export const nodeDepsRun = (
  plan: NodeDepsInstall,
  env: Readonly<Record<string, string>>,
  { signal }: { readonly signal: AbortSignal },
): CommandRun => call('nodeDepsRun', plan, env, { signal });
