import type { PluginIssue } from '../../../format/issues.ts';
import type { ModScanIssue, ModScanIssueCode } from '../scan/index.ts';
import type { ReportEntry } from './validate-report.ts';

/**
 * How the validator prints a refusal of the static analysis (Agent Plugins Extended §10.4.2, Judgment
 * J10.2; oracle 2.1.326): at the path `modules.<entry>`, as `<plugin>: <absolute file>:<line>:
 * <message>; <reminder>`, `<plugin>` the name of the plugin's directory, not the manifest's `name`.
 */

const REMINDER =
  '$ is always spelled $.noun.event(...) at the call site, on is always on("<event>", hook), and next.to always next.to(e, "<tier>")';

/**
 * The rules of the chapter's table, which the lead and the reminder go with; a file that does not
 * parse, an import, a `require`, a `Client` or a hook the scan cannot read keeps its own line.
 */
const IN_THE_TABLE: { readonly [C in ModScanIssueCode]: boolean } = {
  syntax: false,
  'too-deep': false,
  'no-register': true,
  'dynamic-import': false,
  require: false,
  'import-outside': false,
  'import-package': false,
  'import-not-code': false,
  'state-unreadable': false,
  'state-undeclared': true,
  'client-props': false,
  'client-module': false,
  'event-not-literal': true,
  'unknown-event': true,
  'missing-filter': true,
  'hook-unreadable': false,
  'on-shadowed': true,
  'on-twice': true,
  'not-generator': true,
  'dollar-escapes': true,
  'dollar-as-value': true,
};

/** Where the module is: the paths a lead names. */
export interface LeadPlace {
  /** The entry as written in `modules`, `./register.js`. */
  readonly written: string;
  /** The module's path under the plugin root, `hooks/register.js`. */
  readonly entry: string;
  /**
   * The plugin directory's absolute path; absent, the path the user typed (judgment: the lead then
   * names that path and its last segment).
   */
  readonly root: string;
}

const lastSegment = (path: string): string => {
  const trimmed = path.replace(/\/+$/, '');
  return trimmed.slice(trimmed.lastIndexOf('/') + 1) || trimmed;
};

const isScanIssue = (issue: PluginIssue): issue is ModScanIssue =>
  'code' in issue && 'rule' in issue;

/**
 * The entry a validator prints for an error: led as §10.4.2 says when it is a refusal of the table,
 * at the line it is at (line 1 for a rule of the whole file, such as a missing `register`).
 */
export function ledEntry(issue: PluginIssue, place: LeadPlace): ReportEntry {
  if (!isScanIssue(issue) || issue.rule === undefined || !IN_THE_TABLE[issue.code]) {
    return { path: issue.path, message: issue.message };
  }
  const file = `${place.root.replace(/\/+$/, '')}/${place.entry}`;
  const line = String(issue.line ?? 1);
  return {
    path: `modules.${place.written}`,
    message: `${lastSegment(place.root)}: ${file}:${line}: ${issue.rule}; ${REMINDER}`,
  };
}
