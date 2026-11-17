import type { Function as FunctionNode, Node, ObjectExpression, Pattern } from 'acorn';
import {
  isAwait,
  isBlock,
  isCall,
  isDeclarator,
  isIdentifier,
  isObject,
  isObjectPattern,
  isProperty,
  isRestElement,
  isReturn,
  isVariableDeclaration,
  paramName,
  stringOf,
} from './ast.ts';

/**
 * What an `engine.create` hook adds to `$` and withholds from it, read from its source (Agent Plugins
 * Extended §10.4.3, the `adds to $:` and `withholds from $:` lines). Oracle 2.1.325 reads the one
 * form `const { fs, ...beneath } = await next(e); ...; return { ...beneath, voice: voiceOf() }` (or
 * `({ ...(await next(e)), voice: {} })`): the keys destructured beside the rest are withheld, the
 * keys returned beside the spread are added, each value an object, a name or a call. Judgment: a
 * hook in another shape lists nothing, as the oracle's does (its form error is not given).
 */
export interface EngineFold {
  readonly adds: readonly string[];
  readonly withholds: readonly string[];
}

/** `next(e)` or `await next(e)` with the hook's own `next`. */
function isNextCall(node: Node | null | undefined, next: string): boolean {
  const inner = isAwait(node) ? node.argument : node;
  return isCall(inner) && isIdentifier(inner.callee) && inner.callee.name === next;
}

/** A value the form takes: an object literal, a name or a call (`undefined` is none: oracle 2.1.325). */
const isFormValue = (node: Node): boolean =>
  isObject(node) || (isIdentifier(node) && node.name !== 'undefined') || isCall(node);

const keyOf = (node: Node): string | undefined => (isIdentifier(node) ? node.name : stringOf(node));

/** The keys an object literal adds beside its one spread of `base`, or `undefined` out of form. */
function addedKeys(
  object: ObjectExpression,
  spreads: (node: Node) => boolean,
): string[] | undefined {
  const keys: string[] = [];
  let spread = false;
  for (const member of object.properties) {
    if (!isProperty(member)) {
      if (!spreads(member.argument)) return undefined;
      spread = true;
      continue;
    }
    const key = member.computed ? undefined : keyOf(member.key);
    if (key === undefined || !isFormValue(member.value)) return undefined;
    keys.push(key);
  }
  return spread ? keys : undefined;
}

/** `const <pattern> = await next(e)`: the name of what is spread back, and the keys withheld. */
function beneathOf(pattern: Pattern): { base: string; withholds: string[] } | undefined {
  if (isIdentifier(pattern)) return { base: pattern.name, withholds: [] };
  if (!isObjectPattern(pattern)) return undefined;
  const withholds: string[] = [];
  let base: string | undefined;
  for (const member of pattern.properties) {
    if (isRestElement(member)) {
      base = isIdentifier(member.argument) ? member.argument.name : undefined;
      continue;
    }
    const key = member.computed ? undefined : keyOf(member.key);
    if (key === undefined) return undefined;
    withholds.push(key);
  }
  return base === undefined ? undefined : { base, withholds };
}

function blockFold(statements: readonly Node[], next: string): EngineFold | undefined {
  let beneath: { base: string; withholds: string[] } | undefined;
  for (const statement of statements) {
    if (!isVariableDeclaration(statement)) continue;
    const declarator = statement.declarations.find(
      (item) => isDeclarator(item) && isNextCall(item.init, next),
    );
    if (declarator !== undefined) beneath = beneathOf(declarator.id);
  }
  const last = statements.at(-1);
  if (beneath === undefined || !isReturn(last)) return undefined;
  const returned = last.argument;
  const { base, withholds } = beneath;
  const adds = isObject(returned)
    ? addedKeys(returned, (node) => isIdentifier(node) && node.name === base)
    : undefined;
  return adds === undefined ? undefined : { adds, withholds };
}

/** The fold of one `engine.create` hook, or `undefined` when it is not in the form. */
export function engineFold(fn: FunctionNode): EngineFold | undefined {
  const next = paramName(fn, 2);
  if (next === undefined) return undefined;
  const body: Node = fn.body;
  if (isBlock(body)) return blockFold(body.body, next);
  if (!isObject(body)) return undefined;
  const adds = addedKeys(body, (node) => isNextCall(node, next));
  return adds === undefined ? undefined : { adds, withholds: [] };
}
