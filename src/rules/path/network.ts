import type { PathContext, PreparedCall } from '../types.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * Network paths (permissions.md, "Network paths"): "A network path is one that can reach another
 * computer: on Windows, a UNC path such as `\\server\share\file`, and on macOS and Linux, a `/net`
 * automount path such as `/net/fileserver/notes.txt`." Reading one always prompts, whatever an allow
 * rule or a hook says; the host asks `readsNetworkPath` before it trusts a rule.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/path/network.rs`, docs/RUST.md).
 */
const call = coreModule('rules/path/network');

export const isNetworkPath = (path: string, platform: PathContext['platform']): boolean =>
  call('isNetworkPath', path, platform);

/**
 * Whether `call` is a read (Read, Grep, Glob) of a network path, by the path it names or any path it
 * resolves to. Other tools have their own checks.
 */
export const readsNetworkPath = (prepared: PreparedCall, paths: PathContext): boolean =>
  call('readsNetworkPath', prepared, paths);
