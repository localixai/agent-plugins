/**
 * The core's plugin variables and subprocess configuration (Agent Plugins 1.1.0, `spec/1.1.0.md`):
 * core §9.1 (`PLUGIN_ROOT`, `PLUGIN_DATA` set last over the base environment and a server's `env`,
 * by the platform's name rules; the plugin root as the default working directory), §9.2 (one
 * non-recursive pass over `args`, `env` values and `cwd`; never `command`; other `${...}` literal),
 * §7.2.1 (`cwd` expanded before it is resolved, contained in its base; `./` command resolved against
 * the root). Judgment-style tests outside `spec(...)` (docs/SPEC.md, "How the conventions map").
 */
import { describe, expect, test } from 'bun:test';
import {
  coreEnvironment,
  coreHandlerProcess,
  coreServerProcess,
  expandPlaceholders,
  resolveCwd,
} from '../../../src/entry/core.ts';

const paths = {
  root: '/home/alex/.agents/plugins/devtools',
  data: '/home/alex/.agents/plugins/data/devtools',
};

describe('core §9.2: placeholder expansion', () => {
  test('every exact occurrence of either placeholder is replaced', () => {
    expect(expandPlaceholders('${PLUGIN_ROOT}/a:${PLUGIN_DATA}/b:${PLUGIN_ROOT}', paths)).toBe(
      `${paths.root}/a:${paths.data}/b:${paths.root}`,
    );
  });

  test('the pass is not recursive: replaced text is never scanned again', () => {
    const tricky = { root: '/r/${PLUGIN_DATA}', data: '/d' };
    expect(expandPlaceholders('${PLUGIN_ROOT}', tricky)).toBe('/r/${PLUGIN_DATA}');
  });

  test('unrecognised placeholder-like text stays literal', () => {
    expect(
      expandPlaceholders('${HOME} $PLUGIN_ROOT ${plugin_root} ${CLAUDE_PLUGIN_ROOT}', paths),
    ).toBe('${HOME} $PLUGIN_ROOT ${plugin_root} ${CLAUDE_PLUGIN_ROOT}');
  });
});

describe('core §7.2.1: cwd is expanded, then resolved and contained', () => {
  test('each form resolves under its base', () => {
    expect(resolveCwd('./data', paths)).toEqual({
      cwd: { base: 'PLUGIN_ROOT', below: 'data', path: `${paths.root}/data` },
    });
    expect(resolveCwd('${PLUGIN_ROOT}', paths)).toEqual({
      cwd: { base: 'PLUGIN_ROOT', below: '.', path: paths.root },
    });
    expect(resolveCwd('${PLUGIN_DATA}/cache/../x', paths)).toEqual({
      cwd: { base: 'PLUGIN_DATA', below: 'x', path: `${paths.data}/x` },
    });
  });

  test('a value that climbs out of its base after resolution is refused', () => {
    expect('problem' in resolveCwd('${PLUGIN_DATA}/..', paths)).toBe(true);
    expect('problem' in resolveCwd('./x/../..', paths)).toBe(true);
  });
});

describe('core §9.1: the subprocess environment', () => {
  test('env overlays the base, then PLUGIN_ROOT and PLUGIN_DATA replace any same-name entry', () => {
    const env = coreEnvironment(
      { paths, base: { PATH: '/bin', PLUGIN_ROOT: '/wrong', KEEP: '1' } },
      { KEEP: '2' },
    );
    expect(env).toEqual({
      PATH: '/bin',
      KEEP: '2',
      PLUGIN_ROOT: paths.root,
      PLUGIN_DATA: paths.data,
    });
  });

  test('on Windows names compare without case', () => {
    const env = coreEnvironment(
      { paths, base: { Path: 'C:\\bin', plugin_data: 'x' }, platform: 'windows' },
      { PATH: 'C:\\tools' },
    );
    expect(env).toEqual({ PATH: 'C:\\tools', PLUGIN_ROOT: paths.root, PLUGIN_DATA: paths.data });
  });
});

describe('core §7.2.1, §9: what a host launches', () => {
  test('a stdio server: ./ command under the root, args, env and cwd expanded, root by default', () => {
    const launched = coreServerProcess(
      {
        type: 'stdio',
        command: './bin/validator',
        args: ['--data', '${PLUGIN_DATA}/validator'],
        env: { CONFIG: '${PLUGIN_ROOT}/config.json' },
      },
      { paths },
    );
    expect(launched).toEqual({
      process: {
        command: `${paths.root}/bin/validator`,
        args: ['--data', `${paths.data}/validator`],
        cwd: paths.root,
        env: {
          CONFIG: `${paths.root}/config.json`,
          PLUGIN_ROOT: paths.root,
          PLUGIN_DATA: paths.data,
        },
      },
    });
  });

  test('a bare command is passed as written, for the platform search', () => {
    const launched = coreServerProcess(
      { type: 'stdio', command: 'npx', cwd: '${PLUGIN_DATA}' },
      { paths },
    );
    expect('process' in launched && launched.process.command).toBe('npx');
    expect('process' in launched && launched.process.cwd).toBe(paths.data);
  });

  test('a hook handler runs in the plugin root with only its args expanded', () => {
    expect(
      coreHandlerProcess({ command: './hooks/check', args: ['${PLUGIN_ROOT}/p'] }, { paths }),
    ).toEqual({
      command: `${paths.root}/hooks/check`,
      args: [`${paths.root}/p`],
      cwd: paths.root,
      env: { PLUGIN_ROOT: paths.root, PLUGIN_DATA: paths.data },
    });
  });
});
