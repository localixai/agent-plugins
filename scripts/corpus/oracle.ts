/**
 * The validation verdicts: what `claude plugin validate --json <dir>` of one Claude Code release
 * reported for each corpus plugin, recorded by Agent Plugins Extended (`bun run corpus:oracle` in
 * its `extend/tools/`) under `<corpus>/oracle/<release>/`. This repository reads them and never
 * runs the validator for them.
 */
import { join } from 'node:path';
import { z } from 'zod';
import { CORPUS_DIR, FILL } from './location.ts';
const findingSchema = z.object({
  severity: z.enum(['error', 'warning']),
  /** The file the finding is about, relative to the plugin root. */
  file: z.string(),
  path: z.string(),
  message: z.string(),
});
export type OracleFinding = z.infer<typeof findingSchema>;

const reportSchema = z.object({
  ran: z.boolean(),
  findings: z.array(findingSchema),
  failure: z.string().optional(),
});
export type OracleReport = z.infer<typeof reportSchema>;

/**
 * What `claude plugin validate --json` of `release` said about one plugin, as the spec's
 * `corpus:oracle` recorded it; a plugin it has no verdict for is reported as not run.
 */
export async function oracle(release: string, key: string): Promise<OracleReport> {
  const cache = Bun.file(join(CORPUS_DIR, 'oracle', release, `${key}.json`));
  const cached = reportSchema.safeParse(await cache.json().catch(() => undefined));
  return cached.success
    ? cached.data
    : {
        ran: false,
        findings: [],
        failure: `no ${release} verdict: \`bun run corpus:oracle\`, ${FILL}`,
      };
}
