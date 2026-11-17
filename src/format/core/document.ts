import { isRecord } from '../../protocol/is-record.ts';
import {
  CORE_VERSIONS,
  coreSchemaVersion,
  supportedCoreVersion,
  type CoreDocument,
  type CoreVersion,
} from './versions.ts';

/**
 * The outer object of `mcp.json` and `hooks.json` (core §7.2.1, §7.3.1, §10.1): exactly `$schema`
 * and one member (`mcpServers`, `hooks`) that is an object; a `$schema` the kit implements, of the
 * same Agent Plugins version as `plugin.json`. Any failure disables that component type only (core
 * §7.2.2 item 2, §7.3.2 item 2).
 */

/** The member the document holds, or why the whole component is disabled. */
export function documentMembers(
  raw: unknown,
  document: Exclude<CoreDocument, 'plugin'>,
  member: string,
  manifestVersion: CoreVersion,
): { readonly members: Readonly<Record<string, unknown>> } | { readonly problem: string } {
  if (!isRecord(raw)) return { problem: `${document}.json is not a JSON object` };
  const named = coreSchemaVersion(raw.$schema, document);
  if (named === undefined) {
    return { problem: `"$schema" must be the canonical ${document}.json identifier` };
  }
  const version = supportedCoreVersion(named);
  if (version === undefined) {
    return {
      problem: `Unsupported Agent Plugins version ${named}: this client implements ${CORE_VERSIONS.join(', ')}`,
    };
  }
  if (version !== manifestVersion) {
    return {
      problem: `targets Agent Plugins ${String(version)}, but plugin.json targets ${String(manifestVersion)}`,
    };
  }
  const extra = Object.keys(raw).find((key) => key !== '$schema' && key !== member);
  if (extra !== undefined) return { problem: `unknown top-level field "${extra}"` };
  const members = raw[member];
  if (!isRecord(members)) return { problem: `"${member}" must be an object` };
  return { members };
}
