/**
 * errors.md § Custom output styles can't be selected over Remote Control, § Output styles are saved
 * to local settings which this session doesn't load (Agent Plugins Extended §6.2 item 4). Both are
 * docs paragraphs beyond the requirement snapshot, so they are tested outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import { readOutputStyle } from '../../src/format/output-style.ts';
import {
  CUSTOM_STYLES_NOT_SELECTABLE,
  LOCAL_SETTINGS_NOT_LOADED,
  outputStyleCommand,
} from '../../src/format/output-style-command.ts';
import { resolveOutputStyle } from '../../src/format/output-style-select.ts';

const terse = readOutputStyle('output-styles/terse.md', '---\nname: Terse\n---\nBe terse.', {
  plugin: 'writing-kit',
}).style;
const styles = [terse];
const current = resolveOutputStyle({ styles });

describe('/output-style over Remote Control or a relayed message', () => {
  test('lists only the built-in styles, with the notice', () => {
    const reply = outputStyleCommand({ styles, current, relayed: true });
    expect(reply.choices?.map((choice) => choice.id)).toEqual([
      'default',
      'Proactive',
      'Concise',
      'Explanatory',
      'Learning',
    ]);
    expect(reply.notice).toBe(CUSTOM_STYLES_NOT_SELECTABLE);
    expect(CUSTOM_STYLES_NOT_SELECTABLE).toBe(
      "Custom output styles can't be selected over Remote Control or from a relayed message. Select one in the session itself, or pick a built-in style here.",
    );
  });

  test('a custom style, a plugin style included, gets the unknown-name reply and the notice', () => {
    const reply = outputStyleCommand({
      styles,
      current,
      relayed: true,
      argument: 'writing-kit:Terse',
    });
    expect(reply).toEqual({ unknown: 'writing-kit:Terse', notice: CUSTOM_STYLES_NOT_SELECTABLE });
  });

  test('a built-in style is still selected', () => {
    const reply = outputStyleCommand({ styles, current, relayed: true, argument: 'concise' });
    expect(reply.selected?.id).toBe('Concise');
    expect(reply.notice).toBeUndefined();
  });

  test('in the session itself the custom style is listed and selected', () => {
    expect(outputStyleCommand({ styles, current }).choices?.at(-1)?.id).toBe('writing-kit:Terse');
    expect(outputStyleCommand({ styles, current, argument: 'WRITING-KIT:terse' })).toEqual({
      selected: expect.objectContaining({ id: 'writing-kit:Terse' }),
    });
  });
});

describe('a session whose setting sources leave out local', () => {
  test('a selection is refused with the docs message; nothing is selected', () => {
    const reply = outputStyleCommand({
      styles,
      current,
      argument: 'Concise',
      loadsLocalSettings: false,
    });
    expect(reply).toEqual({ refusal: LOCAL_SETTINGS_NOT_LOADED });
    expect(LOCAL_SETTINGS_NOT_LOADED).toBe(
      "Output styles are saved to local settings (.claude/settings.local.json), which this session doesn't load, so the style can't be changed here.",
    );
  });

  test('the list is still shown', () => {
    const reply = outputStyleCommand({ styles, current, loadsLocalSettings: false });
    expect(reply.choices).toHaveLength(6);
    expect(reply.refusal).toBeUndefined();
  });
});
