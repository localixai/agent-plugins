/**
 * `bun run oracle:settings`: the kit's settings merge and starting mode against what the spec
 * recorded (spec/oracle/settings.json, `recorded.ts`): the Agent SDK's own resolver for every key,
 * and for the starting mode also `claude -p`. The kit is handed each case's files; the recording
 * machine's managed settings are not recorded, so a key they set is not compared, and where there
 * were any the kit gets a managed file that sets nothing (`managedReads`). A list compares as a set: the kit and the SDK concatenate tiers in opposite orders,
 * and the order carries no behaviour (spec/judgments.md, `src/settings/merge.ts` and
 * `src/settings/hooks-merge.ts`). Exits 1 on a difference that is not known (`settings-known.ts`),
 * and on a known one that no longer differs.
 */
import { createPermissionsPolicy, createPolicyView, type PolicyContext } from '../../src/index.ts';
import { loadSettingsLayers } from '../../src/entry/settings.ts';
import { readSettingsRecord, type SettingsCase } from './recorded.ts';
import { KNOWN } from './settings-known.ts';

const record = await readSettingsRecord();
const managed = new Set(record.managed.keys);

const context: PolicyContext = {
  account: 'claudeaiPersonal',
  plan: { pro: false },
  hasManagedSettings: record.managed.present,
  parentMerge: false,
  isFolderTrusted: () => true,
  env: {},
  defaultChannelAllowlist: [],
  officialMarketplaces: ['claude-plugins-official'],
};

/**
 * The managed tier the kit is handed: when the recording machine had managed settings, one managed
 * file that delivers a policy key no case compares (`companyAnnouncements`, empty), so a parent
 * layer meets a managed policy as it did there; else none.
 */
const STAND_IN = { companyAnnouncements: [] };
const managedReads = async () =>
  record.managed.present
    ? {
        file: {
          base: undefined,
          dropIns: { files: [{ name: 'recorded.json', read: { value: STAND_IN } }] },
        },
      }
    : {};

/** A value with object keys sorted and every list as a sorted set, as the spec records the SDK's. */
const canonical = (value: unknown): unknown => {
  if (Array.isArray(value)) {
    const items = value.map(canonical).map((item) => JSON.stringify(item));
    return [...new Set(items)].toSorted().map((item) => JSON.parse(item));
  }
  if (value === null || typeof value !== 'object') return value;
  const entries = Object.entries(value).map(([key, inner]) => [key, canonical(inner)] as const);
  return Object.fromEntries(entries.toSorted(([a], [b]) => a.localeCompare(b)));
};
const shown = (value: unknown): string => JSON.stringify(canonical(value)) ?? 'undefined';
const text = (file: unknown): string => (typeof file === 'string' ? file : JSON.stringify(file));

/** The kit's value of each key, and the mode a headless session starts in. */
async function kitSide(one: SettingsCase): Promise<Record<string, string>> {
  const raws: Record<string, string> = {};
  for (const tier of ['user', 'project', 'local', 'parent'] as const) {
    if (one[tier] !== undefined) raws[tier] = text(one[tier]);
  }
  const loaded = await loadSettingsLayers(
    { read: async (scope) => raws[scope], managed: managedReads },
    { signal: new AbortController().signal },
  );
  const view = createPolicyView(loaded.layers);
  const start = createPermissionsPolicy(view, context).startingMode({
    surface: 'headless',
    autoAvailable: true,
  });
  const started = start.outcome === 'started' ? start.mode : 'refused';
  return Object.fromEntries(
    one.keys.map((key) => [key, key === 'defaultMode' ? started : shown(view.get(key as never))]),
  );
}

let failures = 0;
for (const one of record.cases) {
  const kit = await kitSide(one);
  const keys = one.keys.filter((key) => !managed.has(key));
  const sdkOf = (key: string) => (key === 'defaultMode' ? text(one.sdk[key]) : shown(one.sdk[key]));
  const differs = keys.filter((key) => sdkOf(key) !== kit[key]);
  const known = KNOWN[one.name];
  const cliDiffers = one.cli !== undefined && one.cli !== kit['defaultMode'];
  const verdict = differs.length === 0 ? 'agrees' : known ? 'known' : 'DIFFERS';
  const stale = known !== undefined && differs.length === 0;
  if (verdict === 'DIFFERS' || stale || cliDiffers) failures++;
  const cliNote =
    one.cli === undefined ? '' : cliDiffers ? `  cli=DIFFERS(${one.cli})` : '  cli=agrees';
  const skipped = one.keys.length > keys.length ? '  (managed keys not compared)' : '';
  console.log(`${one.name}: sdk=${stale ? 'STALE known' : verdict}${cliNote}${skipped}`);
  for (const key of differs) console.log(`    ${key}: sdk ${sdkOf(key)} kit ${kit[key]}`);
  if (known && differs.length > 0) console.log(`    (${known})`);
}
console.log(
  `\n${record.cases.length} cases recorded by Claude Code ${record.claudeCodeVersion} and the Agent SDK ${record.sdkVersion}, ${failures} failing`,
);
process.exit(failures > 0 ? 1 : 0);
