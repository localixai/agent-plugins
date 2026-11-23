import type { CallExpression, Node } from 'acorn';
import { isIdentifier, isObject, isProperty, stringOf } from './ast.ts';
import type { Found } from './found.ts';

/**
 * The hosts a module's `$.http.fetch` and `$.audio.play({ url })` calls spell, and which of the two
 * methods it calls with a URL the scan cannot read (Agent Plugins Extended §10.19.5 item 1,
 * `e.uses.hosts`; §10.4.3, the `Hosts:` line; Claude Code 2.1.330).
 */
const URL_ARGUMENT: Readonly<Record<string, (call: CallExpression) => Node | undefined>> = {
  'http.fetch': (call) => call.arguments[0],
  'audio.play': (call) => {
    const [options] = call.arguments;
    if (!isObject(options)) return options;
    for (const property of options.properties) {
      if (!isProperty(property) || property.computed) continue;
      const key = isIdentifier(property.key) ? property.key.name : stringOf(property.key);
      if (key === 'url') return property.value;
    }
    return undefined;
  },
};

/** The host of a URL written out whole, or `undefined` when it is not one. */
function hostOf(text: string | undefined): string | undefined {
  if (text === undefined || !URL.canParse(text)) return undefined;
  return new URL(text).hostname || undefined;
}

/** Notes the host a `$.http.fetch` or `$.audio.play` call spells, or the method when it spells none the scan reads. */
export function recordHost(found: Found, name: string, call: CallExpression): void {
  const argumentOf = Object.hasOwn(URL_ARGUMENT, name) ? URL_ARGUMENT[name] : undefined;
  if (argumentOf === undefined) return;
  const node = argumentOf(call);
  // `$.audio.play` of a file plays no URL: no host and nothing unread.
  if (node === undefined && name === 'audio.play') return;
  const host = hostOf(stringOf(node));
  if (host === undefined) found.hostsUnread.add(name);
  else found.hostsNamed.add(host);
}
