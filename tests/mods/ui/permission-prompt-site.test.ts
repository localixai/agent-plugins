/**
 * Agent Plugins Extended §10.21.1 item 6 ("The permission prompt is not a render site. A client MUST
 * NOT offer a mod a way to change what it shows") and §14.9.3 ("The permission prompt appears as the
 * client draws it"). The 2.1.330 spec carries no requirement id for it, so the test runs outside
 * `spec(...)`; the question dialog, `AskUserQuestion`, is a site (`mods.site.AskUserQuestion`).
 */
import { expect, test } from 'bun:test';
import { RENDER_SITE_NAMES } from '../../../src/protocol/mods-ui-names.ts';

test('no render site is the permission prompt, so a mod cannot change what it shows', () => {
  expect(RENDER_SITE_NAMES.filter((name) => /permission/i.test(name))).toEqual([]);
});

test('the question dialog is a render site', () => {
  expect(RENDER_SITE_NAMES).toContain('AskUserQuestion');
});
