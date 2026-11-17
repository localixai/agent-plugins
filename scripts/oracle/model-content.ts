/**
 * What the model reads when a plugin's skill or command is invoked, the kit's side. Each case of
 * spec/oracle/model-content/<case>/ is a tiny plugin (`plugin/`) and a `case.json`; the spec
 * recorded the user text blocks Claude Code sent for it, normalized, in `expected.json`
 * (`recorded.ts`). `kitExpansion` gives the kit's `expandSkill` text for the same invocation,
 * normalized the same way; tests/skills/model-content-oracle.test.ts compares the two.
 */
import { basename, join } from 'node:path';
import {
  createPluginHost,
  createPolicyView,
  loadPlugin,
  readSettingsLayer,
} from '../../src/index.ts';
import { pluginDataId } from '../../src/entry/loading.ts';
import {
  fakeEnvironment,
  locations,
  recordingHttp,
  scriptedRunner,
} from '../../src/testing/index.ts';
import { diskFiles } from '../corpus/disk-files.ts';
import { MODEL_CONTENT_DIR, readModelCase } from './recorded.ts';

const KIT = {
  home: '/home/user',
  project: '/work/app',
  plugins: '/plugins',
  data: '/data',
} as const;
const SESSION_ID = 'oracle-session';

/**
 * A text with the session's own paths as the placeholders the spec records: `<PLUGIN_ROOT>`,
 * `<PLUGIN_DATA>`, `<CWD>`, `<SESSION_ID>`.
 */
function normalize(text: string, paths: { pluginRoot: string; pluginData: string }): string {
  return text
    .split(paths.pluginData)
    .join('<PLUGIN_DATA>')
    .split(paths.pluginRoot)
    .join('<PLUGIN_ROOT>')
    .split(KIT.project)
    .join('<CWD>')
    .replaceAll(SESSION_ID, '<SESSION_ID>');
}

/** What the kit's session gives the model for the case's invocation, normalized like the oracle's. */
export async function kitExpansion(
  name: string,
): Promise<{ ok: boolean; text: string; issues: string[] }> {
  const dir = join(MODEL_CONTENT_DIR, name, 'plugin');
  const input = await readModelCase(name);
  const { plugin } = await loadPlugin({ files: diskFiles(dir), fallbackName: basename(dir) });
  if (plugin === undefined) return { ok: false, text: '', issues: ['the plugin did not load'] };
  const { layer } = readSettingsLayer('user', input.settings ?? {});
  const id = `${plugin.name}@inline`;
  const host = createPluginHost({
    plugins: [{ id, plugin }],
    ports: {
      commands: scriptedRunner(),
      http: recordingHttp(),
      locations: locations(KIT),
      environment: fakeEnvironment(),
    },
    settings: createPolicyView([layer]),
  });
  const session = host.openSession({
    sessionId: SESSION_ID,
    cwd: KIT.project,
    permissionMode: 'default',
    transcriptPath: '/work/oracle.jsonl',
  });
  const expansion = await session.expandSkill(input.skill, {
    arguments: input.arguments,
    signal: AbortSignal.timeout(30_000),
  });
  const issues = expansion.issues.map((issue) => issue.message);
  if (!expansion.ok) return { ok: false, text: '', issues };
  const pluginData = `${KIT.data}/${pluginDataId(id)}`;
  return {
    ok: true,
    text: normalize(expansion.text, { pluginRoot: `${KIT.plugins}/${id}`, pluginData }),
    issues,
  };
}
