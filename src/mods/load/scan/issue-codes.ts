import { error, type PluginIssue } from '../../../format/issues.ts';

/**
 * The one list of what a hooks module scan refuses (plugins/mods/create.md, "Check what Claude Code
 * reads from your mod", and the rules the scan relies on), so a host tells the issues apart by
 * `code` rather than by the words of `message`, which are Claude Code's where the docs quote them
 * and the kit's elsewhere, and may change:
 *
 * - `syntax`: the file does not parse.
 * - `too-deep`: its scopes nest past the scan's limit.
 * - `no-register`: an entry module exports no `register` that takes `on`.
 * - `dynamic-import`, `require`: an `import()` or `require()` call.
 * - `import-outside`: a relative import that leaves the plugin directory.
 * - `import-package`: an import of anything but a relative file or `claude-code`.
 * - `import-not-code`: a relative import of a file whose name does not end like code.
 * - `state-unreadable`: a `$.state` atom whose plugin and key the scan cannot read.
 * - `state-undeclared`: an atom of a plugin's key the manifest does not declare.
 * - `client-props`: a `Client` whose props are no object literal or name no `module`.
 * - `client-module`: a `Client` `module` that is no string literal, is not relative, or leaves the
 *   plugin's folder.
 * - `event-not-literal`, `unknown-event`: `on(...)` with a computed event, or a name that is no event.
 * - `missing-filter`: a telemetry event without `{ to: 'collector' }`.
 * - `hook-unreadable`: the hook passed to `on` is not a function the scan can read.
 * - `on-shadowed`: `on` is declared again inside `register`.
 * - `on-twice`: `on` is called twice for one event without a matcher.
 * - `not-generator`: a hook on a streaming event that is not an async generator.
 * - `dollar-escapes`: `$` is passed to a function the scan cannot follow.
 * - `dollar-as-value`: `$` or a namespace of it is used as a value, or indexed with a computed name.
 */
export const MOD_SCAN_ISSUE_CODES = [
  'syntax',
  'too-deep',
  'no-register',
  'dynamic-import',
  'require',
  'import-outside',
  'import-package',
  'import-not-code',
  'state-unreadable',
  'state-undeclared',
  'client-props',
  'client-module',
  'event-not-literal',
  'unknown-event',
  'missing-filter',
  'hook-unreadable',
  'on-shadowed',
  'on-twice',
  'not-generator',
  'dollar-escapes',
  'dollar-as-value',
] as const;
export type ModScanIssueCode = (typeof MOD_SCAN_ISSUE_CODES)[number];

/** A scan issue: every one is an error, its message opening with the file. */
export interface ModScanIssue extends PluginIssue {
  readonly code: ModScanIssueCode;
  /** The rule's own text, without the file before it: what the validator's lead wraps (J10.2). */
  readonly rule?: string;
  /** The 1-based line of the source the issue is at, when one place in it breaks the rule. */
  readonly line?: number;
}

export const scanIssue = (
  code: ModScanIssueCode,
  file: string,
  message: string,
  line?: number,
): ModScanIssue => ({
  ...error(file, `${file}: ${message}`),
  code,
  rule: message,
  ...(line === undefined ? {} : { line }),
});
