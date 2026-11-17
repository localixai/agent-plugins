/**
 * The wording of the reasons `claude plugin validate` gives for the rules of create.md § Check what
 * Claude Code reads from your mod, where the page names no message (it does for `$.ui is used as a
 * value`, a dynamic import, `require`, a shadowed `on` and a non-literal event name, which
 * check-reads.test.ts asserts under `spec(...)`). These are the kit's own words (spec/judgments.md),
 * not the official CLI's, so they are tested outside `spec(...)`.
 */
import { expect, test } from 'bun:test';
import { validateMod } from '../../../src/mods/load/commands/validate.ts';
import { memoryFiles } from '../../support/ports.ts';

const wrap = (body: string): string =>
  `export function register(on) { on('tool.call', async ($, e, next) => { ${body}; return next(e) }) }`;

async function reason(module: string): Promise<string> {
  const files = memoryFiles({
    '.claude-plugin/plugin.json': JSON.stringify({
      name: 'first-mod',
      version: '0.1.0',
      description: 'd',
      author: { name: 'Your Name' },
    }),
    'hooks/hooks.json': JSON.stringify({ modules: ['./register.js'] }),
    'hooks/register.js': module,
  });
  const report = await validateMod({ files, target: './first-mod' });
  const errors = report.lines.filter((line) => line.startsWith('  ✘'));
  expect(errors).toHaveLength(1);
  return errors[0] ?? '';
}

const TOP_LEVEL =
  'which the scan cannot read: pass $ only to a function declared at the top level of this file';

test('`$` passed to a method, an inner function and an imported function name where it went', async () => {
  expect(await reason(wrap('({ f($) {} }).f($)'))).toContain(
    `$ is passed to a method, ${TOP_LEVEL}`,
  );
  expect(await reason(wrap('const f = ($) => $.fs.read("x"); f($)'))).toContain(
    `$ is passed to f, ${TOP_LEVEL}`,
  );
  expect(await reason(`import { helper } from './helper.js'\n${wrap('helper($)')}`)).toContain(
    `$ is passed to helper, ${TOP_LEVEL}`,
  );
});

test('destructuring `$` and a computed name on it take the words of 2.1.325 (Agent Plugins Extended J10.2)', async () => {
  expect(await reason(wrap('const { ui } = $'))).toContain(
    ':1: $ itself is bound to a name (bound, passed, spread, returned or read); $ is always spelled',
  );
  expect(await reason(wrap("const n = 'ui'; $[n].log('x')"))).toContain(
    ':1: a computed or optional member access on $; $ is always spelled',
  );
});

test('a bare import and an import that leaves the plugin directory each name the specifier', async () => {
  const head = (line: string): string => `${line}\nexport function register(on) {}`;
  expect(await reason(head("import x from 'left-pad'"))).toContain(
    'cannot import "left-pad" (from hooks/register.js): a hooks module imports its own files by relative path and "claude-code", nothing else',
  );
  expect(await reason(head("import x from '../../outside.js'"))).toContain(
    'imports "../../outside.js", outside the plugin directory: a hooks module imports only files',
  );
});
