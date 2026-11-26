import type { CoreManifest } from '../../format/core/manifest.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { ConversionNote } from '../types.ts';

/**
 * `.claude-plugin/plugin.json` from a core manifest (Agent Plugins Extended §18.4.2 item 1): the
 * nine shared fields copied, `$schema` dropped, `extensions` reported. An `author` without `name` and
 * a `homepage` that is no URL fail in Claude Code (§2.2): each is left out and reported, never
 * invented. The name is never changed (§18.4.1 item 2): one `claude plugin validate` refuses (a
 * reserved name) stops the conversion, since its output could not pass the validator (item 3;
 * judgment, spec/judgments.md), and its warnings (not kebab-case, J18.19) are reported.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/to_claude_code/manifest.rs`,
 * docs/RUST.md).
 */
const call = coreModule('convert/to-claude-code/manifest');

export const CLAUDE_MANIFEST: string = call('CLAUDE_MANIFEST');

/** The nine fields both manifests define, in the order the converter writes them (§18.3.1). */
export const SHARED_FIELDS: readonly [
  'name',
  'displayName',
  'version',
  'description',
  'author',
  'homepage',
  'repository',
  'license',
  'keywords',
] = call('SHARED_FIELDS');

export interface ManifestPart {
  readonly value: Readonly<Record<string, unknown>>;
  readonly notes: readonly ConversionNote[];
}

export const claudeCodeManifest = (manifest: CoreManifest): ManifestPart =>
  call('claudeCodeManifest', manifest);
