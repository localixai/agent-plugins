/**
 * The load messages of a plugin's component paths, as Claude Code shows them in the `/plugin`
 * Errors tab and `claude plugin list` (plugins/manifest-reference.md, "Containment and existence"
 * and "How each key combines with its default location"; errors.md, "Path escapes plugin
 * directory"). `component` is the field that declared the path (`commands`, `hooks`, ...).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/messages.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';

const call = coreModule('format/messages');

/** `<component> path not found: <path>`. */
export const pathNotFound = (component: string, path: string): string =>
  call('pathNotFound', component, path);

/**
 * The load error of a declared component path that points at nothing, with its guidance: the docs
 * name `commands`, `skills`, `agents` and `hooks`; the kit gives it every component path the
 * loader checks (judgment, spec/judgments.md). `mcpServers` keeps its own reader's issue.
 */
export const missingPath = (component: string, path: string): PluginIssue =>
  call('missingPath', component, path);

/** The error for a `skills` entry that is not a directory ("Path-only fields": "`skills` entries must be directories"). */
export const skillsEntryNotDirectory = (path: string): string =>
  call('skillsEntryNotDirectory', path);

/** The error for an `agents` entry that is not a file ("Path-only fields": "`agents` entries must be `.md` files"). */
export const agentsEntryNotFile = (path: string): string => call('agentsEntryNotFile', path);

/** The warning for a default folder a replacing manifest key leaves unscanned. */
export const defaultFolderIgnored = (folder: string, key: string): string =>
  call('defaultFolderIgnored', folder, key);
