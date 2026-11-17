import type { Node } from 'acorn';
import { isBlock, isFunction } from './ast.ts';

/**
 * A walk over a syntax tree that keeps no recursion: a hooks module may nest code deeply
 * (reference.md, "Limits": 2,000 scopes), and the walk must not depend on the stack to say so.
 */

/** Every child node of `node`, in source order. */
export function childrenOf(node: Node): readonly Node[] {
  const found: Node[] = [];
  for (const value of Object.values(node)) {
    const items: unknown[] = Array.isArray(value) ? value : [value];
    for (const item of items) {
      if (typeof item === 'object' && item !== null && 'type' in item && 'start' in item) {
        // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- an object with `type` and `start` is an acorn node
        found.push(item as Node);
      }
    }
  }
  return found.toSorted((a, b) => a.start - b.start);
}

/** Node types that open a scope: functions, blocks, loops, `catch`, `switch`, classes (a function's own body block is part of the function). */
const SCOPE_TYPES: ReadonlySet<string> = new Set([
  'FunctionDeclaration',
  'FunctionExpression',
  'ArrowFunctionExpression',
  'BlockStatement',
  'StaticBlock',
  'ForStatement',
  'ForInStatement',
  'ForOfStatement',
  'WhileStatement',
  'DoWhileStatement',
  'CatchClause',
  'SwitchStatement',
  'ClassBody',
]);
const FUNCTIONS: ReadonlySet<string> = new Set([
  'FunctionDeclaration',
  'FunctionExpression',
  'ArrowFunctionExpression',
]);

interface Frame {
  readonly node: Node;
  readonly depth: number;
  /** The function whose body block this is, which does not open a scope of its own. */
  readonly bodyOf?: Node;
}

/** The deepest nesting of scopes in a program, counted without recursion. */
export function maxScopeDepth(program: Node): number {
  let deepest = 0;
  const stack: Frame[] = [{ node: program, depth: 0 }];
  for (let frame = stack.pop(); frame; frame = stack.pop()) {
    const { node } = frame;
    const isOwnBody = frame.bodyOf !== undefined && isBlock(node);
    const opens = SCOPE_TYPES.has(node.type) && !isOwnBody;
    const depth = frame.depth + (opens ? 1 : 0);
    deepest = Math.max(deepest, depth);
    const body = FUNCTIONS.has(node.type) ? Reflect.get(node, 'body') : undefined;
    for (const child of childrenOf(node)) {
      stack.push({ node: child, depth, ...(child === body ? { bodyOf: node } : {}) });
    }
  }
  return deepest;
}

/** Children the walk follows: a function's parameters are declarations, so only its body is read. */
const readable = (node: Node): readonly Node[] =>
  isFunction(node) ? [node.body] : childrenOf(node);

/** A walk with a context handed down: the visitor returns the context for the children, or `false` to stop. */
export function walk<C>(
  root: Node,
  context: C,
  visit: (node: Node, context: C) => C | false,
): void {
  const stack: { readonly node: Node; readonly context: C }[] = [{ node: root, context }];
  for (let frame = stack.pop(); frame; frame = stack.pop()) {
    const next = visit(frame.node, frame.context);
    if (next === false) continue;
    for (const child of readable(frame.node).toReversed()) {
      stack.push({ node: child, context: next });
    }
  }
}
