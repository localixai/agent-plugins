import type { z } from 'zod';
import type { Finding } from './finding.ts';
import { pathShapeFindings, type PathKey } from './plugin-paths.ts';
import { zodFindings } from './zod-findings.ts';
import { lspSourcesSchema } from '../lsp-schema.ts';
import { monitorSourcesSchema } from '../monitor-schema.ts';

/**
 * The keys whose inline form is a strict object the validator reads entry by entry (spec §9.1.3,
 * §9.2.2; oracle 2.1.325): the `lspServers` map and the monitors array, at the top level and under
 * `experimental`. The loader reads the same schemas through the manifest's (manifest-schema.ts).
 */
const INLINE_SCHEMAS: Partial<Readonly<Record<PathKey, z.ZodType>>> = {
  lspServers: lspSourcesSchema,
  monitors: monitorSourcesSchema,
};

/**
 * The findings of one path-valued key: an inline LSP map or monitors array that breaks its schema
 * is reported at the path of each problem (`lspServers.go: Unrecognized key: "foo"`,
 * `experimental.monitors: Monitor names must be unique within a plugin`), or as `Invalid input` at
 * the key when no form fits; otherwise the path rules of `plugin-paths.ts`.
 */
export function sourceFindings(key: PathKey, path: string, value: unknown): Finding[] {
  const schema = INLINE_SCHEMAS[key];
  const parsed = schema?.safeParse(value);
  if (parsed !== undefined && !parsed.success) return zodFindings(parsed, path);
  return pathShapeFindings(path, key, value);
}
