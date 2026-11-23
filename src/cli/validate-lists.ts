import { declaredHosts } from '../format/network.ts';
import type { Plugin } from '../loading/plugin-model.ts';
import type { ProvidesKind } from '../mods/load/index.ts';

/**
 * What `claude plugin validate` lists after the findings of a plugin run (spec §17.13.4, Judgment
 * J17.36): `provides`, the plugin's components by kind, and `hosts`, the hosts it names and the parts
 * that could reach others but were not read for them. Neither is a finding: they never change the
 * verdict. A run that cannot list them (a marketplace or a components-only run) gives `null`.
 */

/** The kinds `provides` holds, in Claude Code 2.1.330's order: those of `e.provides` (`PROVIDES_KINDS`). */
export const VALIDATION_PROVIDES_KINDS = [
  'modules',
  'hooks',
  'mcpServers',
  'lspServers',
  'monitors',
  'skills',
  'commands',
  'agents',
  'outputStyles',
  'themes',
  'workflows',
  'bin',
] as const satisfies readonly ProvidesKind[];
export type ValidationProvidesKind = (typeof VALIDATION_PROVIDES_KINDS)[number];

export type ValidationProvides = { readonly [K in ValidationProvidesKind]: readonly string[] };

export interface UnreadPart {
  readonly kind: ValidationProvidesKind;
  readonly name: string;
  /** Why it was not read for hosts: `process`, a part that starts a program. */
  readonly reason: string;
}

export interface ValidationHosts {
  readonly named: readonly {
    readonly host: string;
    readonly sources: readonly { readonly type: string }[];
  }[];
  readonly unread: readonly UnreadPart[];
  /** Whether the manifest declares hosts, and which servers do (2.1.330). */
  readonly declared: { readonly plugin: boolean; readonly servers: readonly string[] };
}

const base = (path: string): string => path.slice(path.lastIndexOf('/') + 1);

/** Each hook handler, named by its event. */
const hookNames = (plugin: Plugin): string[] =>
  Object.entries(plugin.hooks).flatMap(([event, groups]) =>
    (groups ?? []).flatMap((group) => group.hooks.map(() => event)),
  );

/**
 * The components by kind, each named as a session names it (`<plugin>:<skill>`). `modules` are the
 * hooks modules the host's mods loader found, which a `Plugin` does not hold.
 */
export function pluginProvides(
  plugin: Plugin,
  modules: readonly string[] = [],
): ValidationProvides {
  return {
    modules: [...modules],
    hooks: hookNames(plugin),
    mcpServers: Object.keys(plugin.mcpServers),
    lspServers: Object.keys(plugin.lspServers),
    monitors: plugin.monitors.map((monitor) => monitor.name),
    skills: plugin.skills.map((skill) => skill.qualifiedName),
    commands: plugin.commands.map((command) => command.qualifiedName),
    agents: plugin.agents.map((agent) => agent.qualifiedName),
    outputStyles: plugin.outputStyles.map((style) => style.qualifiedName),
    themes: plugin.themes.map((theme) => theme.name),
    workflows: plugin.workflows.map((workflow) => workflow.qualifiedName ?? workflow.name),
    bin: plugin.bin.map(base),
  };
}

/**
 * The parts that start a program, and so could reach hosts the validator cannot read (the kit's set
 * beyond the skills and LSP servers the spec names; spec/judgments.md, J17.36).
 */
const UNREAD_KINDS: readonly ValidationProvidesKind[] = [
  'hooks',
  'mcpServers',
  'lspServers',
  'monitors',
  'skills',
  'bin',
];

/**
 * `spelled` are the hosts the hooks module's `$.http.fetch` and `$.audio.play` calls spell (§10.4.3),
 * each with the source `{ type: 'module' }` (judgment, spec/judgments.md: the spec names `declared`
 * only), after the declared ones; a host both declares and spells has both sources.
 */
export function pluginHosts(
  plugin: Plugin,
  provides: ValidationProvides,
  spelled: readonly string[] = [],
): ValidationHosts {
  const declared = declaredHosts(plugin.manifest?.network);
  const hosts = [...new Set([...declared, ...spelled])];
  const sourcesOf = (host: string) => [
    ...(declared.includes(host) ? [{ type: 'declared' }] : []),
    ...(spelled.includes(host) ? [{ type: 'module' }] : []),
  ];
  return {
    named: hosts.map((host) => ({ host, sources: sourcesOf(host) })),
    unread: UNREAD_KINDS.flatMap((kind) =>
      provides[kind].map((name) => ({ kind, name, reason: 'process' })),
    ),
    declared: { plugin: declared.length > 0, servers: [] },
  };
}

/** What the mods validator says of a plugin's hooks module, in each of its results (`validateMod`'s `contents`). */
export type ModValidationResults = readonly {
  readonly modules?:
    | readonly {
        readonly module: string;
        readonly uses: { readonly hosts?: { readonly named: readonly string[] } };
      }[]
    | null;
}[];

/**
 * `provides` and `hosts` of a plugin run: the plugin's parts, `modules` from the hooks modules the
 * mods validator read, and the hosts the manifest declares and the modules' calls spell.
 */
export function validationLists(
  plugin: Plugin,
  mods: ModValidationResults = [],
): { readonly provides: ValidationProvides; readonly hosts: ValidationHosts } {
  const modules = mods.flatMap((result) => result.modules ?? []);
  const provides = pluginProvides(
    plugin,
    modules.map((one) => one.module),
  );
  const spelled = modules.flatMap((one) => one.uses.hosts?.named ?? []);
  return { provides, hosts: pluginHosts(plugin, provides, spelled) };
}

const NOT_DECLARED =
  'Hosts not declared: list what these parts reach under "network.hosts" in plugin.json';

/** What the `Hosts:` line adds after a host for each source: `(declared)` for the manifest's (§17.13.4). */
const SOURCE_MARKS: Readonly<Record<string, string>> = { declared: ' (declared)' };

/** `api.example.com (declared)` for a host the manifest declares, the bare host for one only code spells. */
const hostEntry = (one: ValidationHosts['named'][number]): string =>
  one.host + (one.sources.map((source) => SOURCE_MARKS[source.type]).find(Boolean) ?? '');

/** The text lines after the findings: `Provides:`, then the `Hosts` lines that apply. */
export function validationListLines(
  provides: ValidationProvides,
  hosts: ValidationHosts,
): readonly string[] {
  const kinds = VALIDATION_PROVIDES_KINDS.filter((kind) => provides[kind].length > 0);
  const counts = new Map<string, number>();
  for (const part of hosts.unread) counts.set(part.kind, (counts.get(part.kind) ?? 0) + 1);
  return [
    ...(kinds.length > 0 ? ['Provides:'] : []),
    ...kinds.map((kind) => `  ❯ ${kind}: ${provides[kind].join(', ')}`),
    ...(hosts.named.length > 0 ? [`Hosts: ${hosts.named.map(hostEntry).join(', ')}`] : []),
    ...(counts.size > 0
      ? [`Hosts not read for: ${[...counts].map(([kind, n]) => `${kind} (${n})`).join(', ')}`]
      : []),
    ...(counts.size > 0 && !hosts.declared.plugin ? [NOT_DECLARED] : []),
  ];
}
