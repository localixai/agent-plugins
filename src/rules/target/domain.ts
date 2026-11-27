import type { ArtifactInput } from '../../protocol/official-tools.ts';
import type { Assert, Equal } from '../../protocol/type-equality.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { SpecifierMatcher } from '../matchers.ts';
import type { PermissionRule, PreparedCall } from '../types.ts';

/**
 * WebFetch domain rules (permissions.md, "WebFetch"): `domain:` is matched against the hostname of
 * the requested URL, case-insensitively, with a trailing `.` stripped from the rule and the
 * hostname. `example.com` is that host only; a leading `*.` is any subdomain at any depth but not
 * the domain itself; a bare `*` is every host; a `*` anywhere else stands for the text between two
 * dots only, so `example.*` is `example.org` and never `example.evil.com`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/target/domain.rs`, docs/RUST.md).
 */
const call = coreModule('rules/target/domain');

/**
 * @public A compile-time drift guard: `read`, the action the core reads an `Artifact` read by, is
 * one of the SDK's `ArtifactInput.action`.
 */
export type ArtifactReadIsAction = Assert<
  Equal<Extract<NonNullable<ArtifactInput['action']>, 'read'>, 'read'>
>;

/**
 * Whether the `domain:` pattern of a WebFetch rule covers the host of `url`. An address in brackets
 * (`[::1]`) covers that address on every port, and `[::1]:443` on port 443 only; a URL that does not
 * parse has no host and matches no pattern. A pattern with non-ASCII labels is compared in the
 * punycode form `URL.hostname` gives the host (spec/judgments.md).
 */
export const matchDomainPattern = (pattern: string, url: string): boolean =>
  call('matchDomainPattern', pattern, url);

/**
 * The matcher of the `domain` rule format: a `WebFetch(domain:...)` rule against the call's `url`.
 * A specifier without the `domain:` prefix matches nothing, because the docs give WebFetch no other
 * form (spec/judgments.md).
 */
export const matchWebFetchRule: SpecifierMatcher = (rule, prepared, ctx) =>
  call('matchWebFetchRule', rule, prepared, ctx);

/**
 * Whether a `WebFetch(domain:...)` rule covers an `Artifact` read (permissions.md, "Allow or deny
 * every fetch"): a `domain:` rule covering `claude.ai`, as the read's URL names it, or the
 * `*.claudeusercontent.com` content host applies to the read in a deny or ask rule. A bare
 * `WebFetch` rule never does, because its tool name is not the call's; the `Artifact` rule is a
 * rule on that tool and matches by name.
 */
export const domainRuleCoversArtifactRead = (
  rule: PermissionRule,
  prepared: PreparedCall,
): boolean => call('domainRuleCoversArtifactRead', rule, prepared);

/** The sandbox's domain lists (sandboxing.md, "Network isolation"). */
export type SandboxList = 'allowed' | 'denied';

/** The host a `WebFetch(domain:...)` rule adds to a sandbox list. */
export interface SandboxDomainEntry {
  readonly list: SandboxList;
  readonly domain: string;
}

/**
 * What a rule changes about the hosts sandboxed commands reach (permissions.md, "Allow or deny every
 * fetch"; sandboxing.md): only a `WebFetch(domain:...)` rule adds its domain to a list, and a bare
 * `WebFetch` rule changes nothing there. A wildcard in a position the sandbox does not honor
 * (`example.*`) still matches fetches and adds nothing. `undefined` is "no entry".
 */
export const sandboxDomainEntry = (rule: PermissionRule): SandboxDomainEntry | undefined =>
  call('sandboxDomainEntry', rule);

/** The two sandbox lists the `WebFetch(domain:...)` rules in `rules` contribute to. */
export const sandboxDomainLists = (
  rules: readonly PermissionRule[],
): { readonly allowedDomains: readonly string[]; readonly deniedDomains: readonly string[] } =>
  call('sandboxDomainLists', rules);
