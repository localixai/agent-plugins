import { error, type PluginIssue } from './issues.ts';

/**
 * The load messages of a plugin's component paths, as Claude Code shows them in the `/plugin`
 * Errors tab and `claude plugin list` (plugins/manifest-reference.md, "Containment and existence"
 * and "How each key combines with its default location"; errors.md, "Path escapes plugin
 * directory"). `component` is the field that declared the path (`commands`, `hooks`, ...).
 */

/** `<component> path escapes plugin directory: <path>`, with where a symlink really leads. */
export const pathEscapes = (
  component: string,
  path: string,
  resolvesTo?: string,
  /** Also outside the marketplace: the form for a plugin's own file in a local marketplace. */
  withMarketplace = false,
): string =>
  resolvesTo === undefined
    ? `${component} path escapes plugin directory: ${path}`
    : `${component} path escapes plugin directory: ${path} — it resolves to ${resolvesTo}, outside the plugin directory${withMarketplace ? ' and its marketplace directory' : ''}`;

/**
 * What `<component> path escapes plugin directory: <path>` adds when the path stays inside but holds a
 * backslash on macOS or Linux (errors.md, "Path escapes plugin directory").
 */
export const BACKSLASH_TAIL =
  ' — its path contains a backslash, which is not resolved reliably on this platform';

/** `<component> path not found: <path>`. */
export const pathNotFound = (component: string, path: string): string =>
  `${component} path not found: ${path}`;

/** The guidance of a declared path that points at nothing (plugins/troubleshooting.md, "commands path not found"). */
const PATH_NOT_FOUND_GUIDANCE =
  'Check that the path in your manifest or marketplace config is correct';

/**
 * The load error of a declared component path that points at nothing, with its guidance: the docs
 * name `commands`, `skills`, `agents` and `hooks`; the kit gives it every component path the
 * loader checks (judgment, spec/judgments.md). `mcpServers` keeps its own reader's issue.
 */
export const missingPath = (component: string, path: string): PluginIssue => ({
  ...error(component, pathNotFound(component, path)),
  guidance: PATH_NOT_FOUND_GUIDANCE,
});

/** The error for a `skills` entry that is not a directory ("Path-only fields": "`skills` entries must be directories"). */
export const skillsEntryNotDirectory = (path: string): string =>
  `skills entries must be directories: ${path}`;

/** The error for an `agents` entry that is not a file ("Path-only fields": "`agents` entries must be `.md` files"). */
export const agentsEntryNotFile = (path: string): string =>
  `agents entries must be .md files: ${path}`;

/** The warning for a default folder a replacing manifest key leaves unscanned. */
export const defaultFolderIgnored = (folder: string, key: string): string =>
  `Default ${folder}/ folder is ignored because the manifest sets "${key}"`;
