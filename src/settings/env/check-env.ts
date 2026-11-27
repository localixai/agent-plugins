/**
 * What a file may hold under `env` (settings-reference.md, `env`: "object mapping variable names to
 * string values"; "Variables Claude Code ignores in `env`"): the variables Claude Code drops from the
 * file's scope are removed, named and never with their value, and so is an entry whose name no
 * process could carry. A value that is not a string is read as JavaScript's `String(value)`, as
 * claude 2.1.321 sets it (`1` is "1", `true` "true", `null` "null", `[1,2]` "1,2"), and is reported
 * so the author sees it (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/env/check_env.rs`, docs/RUST.md).
 */
import type { Repair } from '../repair/support.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/env/check-env');

/** `env`: the entries Claude Code ignores or cannot read are removed, each reported by name. */
export const env: Repair = (value, at) => call('env', value, at);
