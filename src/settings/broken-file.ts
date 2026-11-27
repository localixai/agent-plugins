/** Implemented by the Rust core (`crates/agent-plugins/src/settings/broken_file.rs`, docs/RUST.md). */
import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from '../format/issues.ts';
import type { PolicyScope } from './layers.ts';

const call = coreModule('settings/broken-file');

/**
 * Whether a broken source stops Claude Code (errors.md, "Managed settings document could not be
 * parsed": it "exits with code 1 at startup instead of running without the policy the document
 * carries"). The docs say it of the managed documents; the kit holds the parent settings of an
 * embedding host, also policy, to the same rule (spec/judgments.md). Any other source is skipped.
 */
export const stopsStartup = (scope: PolicyScope): boolean => call('stopsStartup', scope);

/**
 * A source that is present and is not a JSON object: `reason` says why when the text did not parse.
 * A user, project, local or `--settings` file is skipped. A managed document is fatal: the line
 * names the failed source (`source`, the path or profile the host read, when it says) before the
 * message, as errors.md shows it.
 */
export const brokenSource = (scope: PolicyScope, reason?: string, source?: string): PluginIssue =>
  call('brokenSource', scope, reason, source);

/** What `/status` and the start-up dialog call a problem (settings.md, "Fix a broken settings file"). */
export type ProblemKind = 'Settings Error' | 'Settings Warning' | 'Managed settings';

/**
 * Which of the documented kinds a problem found in `scope` is: an entry that fails is a Settings
 * Warning, a file that does not parse a Settings Error, and anything in the policy tier is reported
 * as managed settings, which keep enforcing the rest of the file.
 */
export const problemKind = (scope: PolicyScope, issue: PluginIssue): ProblemKind =>
  call('problemKind', scope, issue);
