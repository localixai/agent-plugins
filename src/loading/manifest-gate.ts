import {
  manifestNotRead,
  pathCouldNotBeChecked,
  reportedCheckCode,
} from '../format/host-errors.ts';
import { error, type PluginIssue } from '../format/issues.ts';
import { STANDARD_LAYOUT } from '../format/layout.ts';
import { linkReach } from '../format/link-rule.ts';
import type { LinkRoots } from '../format/path-escape.ts';
import { resolveFrom, withoutTrailingSlashes, type PathPlatform } from '../format/path-text.ts';
import type { FileStat, PluginFiles } from '../ports/index.ts';

/**
 * What stops a plugin before anything of it is read (spec §17.18.3, §17.18.5): its own directory
 * that the system could not check (`plugin path could not be checked: . (<code>)`), and a
 * `.claude-plugin/plugin.json` the kit cannot confirm is inside the plugin, or for a plugin of a
 * marketplace inside that marketplace: `Plugin not loaded: its manifest <path> was not read,
 * because <reason>`, the reason ending in the error code when the path could not be checked. The
 * component name `plugin` and the reason `it could not be checked (<code>)` are judgments
 * (spec/judgments.md): the specification gives the form of the message, not these words. A link is
 * judged only when the host passes `roots`, as for every other plugin path.
 */
interface GateOptions {
  readonly roots?: LinkRoots;
  readonly platform?: PathPlatform;
  readonly signal?: AbortSignal;
}

const FOLDER = '.claude-plugin';

/** Where a link among the manifest's path leads: through its folder, or the file itself. */
function linkTarget(folder: FileStat | undefined, file: FileStat | undefined): string | undefined {
  if (folder?.isSymlink === true && folder.realPath !== undefined) {
    return `${withoutTrailingSlashes(folder.realPath)}/plugin.json`;
  }
  return file?.isSymlink === true ? file.realPath : undefined;
}

function outsideReason(
  target: string,
  roots: LinkRoots,
  platform: PathPlatform,
): string | undefined {
  const real = resolveFrom(roots.pluginRoot, target, platform);
  if (linkReach({ target: real, ...roots }) !== 'outside') return undefined;
  const marketplace = roots.marketplaceRoot === undefined ? '' : ' and its marketplace directory';
  return `it resolves to ${real}, outside the plugin directory${marketplace}`;
}

/** The plugin folder itself, when the system could not check it. */
async function rootIssue(
  files: PluginFiles,
  signal: AbortSignal | undefined,
): Promise<PluginIssue | undefined> {
  const code = (await files.stat('.', signal))?.checkFailed;
  return code === undefined
    ? undefined
    : error('plugin', pathCouldNotBeChecked('plugin', '.', code));
}

/** Why the manifest is not read: a check that failed, or a link that leads outside. */
function manifestReason(
  folder: FileStat | undefined,
  file: FileStat | undefined,
  options: GateOptions,
): string | undefined {
  const failed = folder?.checkFailed ?? file?.checkFailed;
  if (failed !== undefined) return `it could not be checked (${reportedCheckCode(failed)})`;
  const target = linkTarget(folder, file);
  if (target === undefined || options.roots === undefined) return undefined;
  return outsideReason(target, options.roots, options.platform ?? 'posix');
}

/** The issue that keeps the whole plugin from loading, or `undefined` when it may load. */
export async function manifestGate(
  files: PluginFiles,
  options: GateOptions = {},
): Promise<PluginIssue | undefined> {
  const { signal, roots } = options;
  const root = await rootIssue(files, signal);
  if (root) return root;
  const { location } = STANDARD_LAYOUT.manifest;
  const [folder, file] = await Promise.all([
    files.stat(FOLDER, signal),
    files.stat(location, signal),
  ]);
  const reason = manifestReason(folder, file, options);
  if (reason === undefined) return undefined;
  const shown = roots === undefined ? location : `${roots.pluginRoot}/${location}`;
  return error(location, manifestNotRead(shown, reason));
}
