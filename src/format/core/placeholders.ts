import { resolveSegments } from '../path-text.ts';

/**
 * The two plugin variables of the core (core §9): `${PLUGIN_ROOT}`, the filesystem-resolved plugin
 * root, and `${PLUGIN_DATA}`, the client-managed data directory of the installed instance. Expansion
 * is one non-recursive textual pass over every exact occurrence of either placeholder: text a
 * replacement introduces is never scanned again, and any other `${...}` stays literal (core §9.2).
 */

/** The two directories, absolute and `/`-spelled, as the host resolved them. */
export interface CorePaths {
  readonly root: string;
  readonly data: string;
}

export const CORE_VARIABLES = ['PLUGIN_ROOT', 'PLUGIN_DATA'] as const;
export type CoreVariable = (typeof CORE_VARIABLES)[number];

const PLACEHOLDER = /\$\{(PLUGIN_ROOT|PLUGIN_DATA)\}/g;

const valueOf = (paths: CorePaths): { readonly [V in CoreVariable]: string } => ({
  PLUGIN_ROOT: paths.root,
  PLUGIN_DATA: paths.data,
});

const isVariable = (name: string): name is CoreVariable =>
  CORE_VARIABLES.some((known) => known === name);

/** `text` with both placeholders replaced in one pass (core §9.2). */
export function expandPlaceholders(text: string, paths: CorePaths): string {
  const values = valueOf(paths);
  return text.replace(PLACEHOLDER, (whole: string, name: string) =>
    isVariable(name) ? values[name] : whole,
  );
}

/**
 * The three forms a stdio server's `cwd` may take (core §7.2.1): a `./` path under the plugin root,
 * `${PLUGIN_ROOT}` or a path below it, `${PLUGIN_DATA}` or a path below it. `rest` is what follows
 * the base, before expansion.
 */
interface CwdForm {
  readonly base: CoreVariable;
  readonly rest: string;
}

function cwdForm(cwd: string): CwdForm | undefined {
  if (cwd.startsWith('./')) return { base: 'PLUGIN_ROOT', rest: cwd.slice(2) };
  const base = CORE_VARIABLES.find(
    (name) => cwd === `\${${name}}` || cwd.startsWith(`\${${name}}/`),
  );
  return base === undefined ? undefined : { base, rest: cwd.slice(base.length + 3) };
}

/** A resolved `cwd`: under which directory, the path below it, and the absolute path. */
export interface ResolvedCwd {
  readonly base: CoreVariable;
  /** `/`-separated, no `.` or `..`; `.` for the base itself. */
  readonly below: string;
  readonly path: string;
}

/**
 * Expands, then resolves a `cwd` (core §7.2.1: "Clients MUST expand placeholders before resolving
 * `cwd`"), or says why the server entry is invalid: another form, or a path that climbs out of its
 * base. The containment checked here is textual; whether a link below the base leads out is the
 * loader's question for the plugin root (`loadCorePackage`) and the host's for the data directory.
 */
export function resolveCwd(
  cwd: string,
  paths: CorePaths,
): { readonly cwd: ResolvedCwd } | { readonly problem: string } {
  const form = cwdForm(cwd);
  if (form === undefined) {
    return { problem: 'must begin with "./", "${PLUGIN_ROOT}" or "${PLUGIN_DATA}"' };
  }
  const segments = resolveSegments(expandPlaceholders(form.rest, paths), 'posix');
  if (segments === undefined) return { problem: `leaves ${form.base} after it is resolved` };
  const base = valueOf(paths)[form.base];
  const below = segments.length === 0 ? '.' : segments.join('/');
  return { cwd: { base: form.base, below, path: below === '.' ? base : `${base}/${below}` } };
}

/**
 * Paths to judge a configuration by before the host knows the real ones: absolute paths that hold
 * no `..`, as any real resolved directory does, so containment comes out the same.
 */
export const SYMBOLIC_PATHS: CorePaths = { root: '/plugin-root', data: '/plugin-data' };
