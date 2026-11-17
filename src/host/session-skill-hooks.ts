import { warning, type PluginIssue } from '../format/issues.ts';
import { type HookSource, type RegisteredHandler } from '../hooks/index.ts';
import type { SkillOriginKind } from '../format/skill-types.ts';
import { admitSkillHooks, hookSourceOf, type Expansion } from '../skills/index.ts';
import type { SkillHookLedger } from './session-core.ts';
import type { SkillHooksRegistrar } from './session-hooks.ts';
import type { Entry } from './session-skill-entries.ts';
import { registerHooks } from '../hooks/register.ts';
import { NO_HOOKS_YET } from '../skills/hook-limits.ts';

/**
 * hooks.md, "Hooks in skills and agents": a skill's hooks are registered when it is invoked and
 * keep running for the rest of the session; "Limits on skill hooks": each source of skills registers
 * at most 1,024 hooks and 16,384 matcher characters per session, and a skill past the limit
 * "registers none of its hooks". Judgment: invoking a skill again replaces its own hooks and counts
 * them once (spec/judgments.md).
 */
export function skillHookRegistration(registrar: SkillHooksRegistrar, ledger: SkillHookLedger) {
  const { used, counted } = ledger;
  return (
    entry: Entry,
    hooks: NonNullable<Extract<Expansion, { ok: true }>['hooks']>,
  ): readonly PluginIssue[] => {
    const source = hookSourceOf(entry.invocable.origin);
    const prefix = `skill:${entry.plugin?.id ?? entry.invocable.origin.kind}:${entry.invocable.command}`;
    const others = used.get(source) ?? NO_HOOKS_YET;
    const own = counted.get(prefix) ?? NO_HOOKS_YET;
    const base = {
      hooks: others.hooks - own.hooks,
      matcherChars: others.matcherChars - own.matcherChars,
    };
    const admission = admitSkillHooks(base, hooks);
    if (!admission.admitted) {
      return [
        warning(
          `/${entry.invocable.command}`,
          `Skill "${entry.invocable.command}" registers none of its hooks: ${source} is past its limit of 1,024 hooks or 16,384 matcher characters`,
        ),
      ];
    }
    used.set(source, admission.used);
    counted.set(prefix, {
      source,
      ...(entry.plugin === undefined ? {} : { pluginId: entry.plugin.id }),
      hooks: admission.used.hooks - base.hooks,
      matcherChars: admission.used.matcherChars - base.matcherChars,
    });
    const entries: readonly RegisteredHandler[] = registerHooks(hooks, {
      idPrefix: prefix,
      source: {
        kind: 'skill',
        label: '[skill]',
        ...SKILL_HOOK_SOURCE[entry.invocable.origin.kind],
      },
      ...(entry.plugin === undefined ? {} : { pluginId: entry.plugin.id }),
    });
    registrar.registerSkillHooks(prefix, entries);
    return [];
  };
}

/**
 * The trust and lock facts a skill's frontmatter hooks carry, per origin (hooks.md, "Hooks in skills
 * and agents": a project skill's hooks "follow the same workspace trust rule as hooks in settings
 * files"; "Limits on skill hooks": a plugin-only lock keeps those of "a plugin skill, a bundled
 * skill, or a skill in the managed settings directory"). A skill synced from claude.ai counts as
 * the user's own (settings-reference.md, `strictPluginOnlyCustomization`).
 */
const SKILL_HOOK_SOURCE: {
  readonly [K in SkillOriginKind]: Pick<HookSource, 'scope' | 'managed'>;
} = {
  plugin: {},
  managed: { managed: true },
  user: { scope: 'user' },
  project: { scope: 'project' },
  synced: { scope: 'user' },
  bundled: {},
};

/** The expansion with the warnings of registering its hooks, when it has hooks to register. */
export function withHooks(
  expansion: Expansion,
  register: (
    hooks: NonNullable<Extract<Expansion, { ok: true }>['hooks']>,
  ) => readonly PluginIssue[],
): Expansion {
  if (!expansion.ok || expansion.hooks === undefined) return expansion;
  const issues = register(expansion.hooks);
  return issues.length === 0
    ? expansion
    : { ...expansion, issues: [...expansion.issues, ...issues] };
}
