/**
 * permissions.md § Project allow rules and workspace trust (a background session checks trust for
 * its directory before it starts or restarts: a terminal there shows the dialog, elsewhere the
 * command exits) and errors.md § Workspace not trusted when dispatching a background session (the
 * message and its two variants). Agent Plugins Extended §13.15 item 4.
 */
import { expect, test } from 'bun:test';
import { backgroundSessionTrust } from '../../src/loading/background-trust.ts';
import { spec } from '../support/spec.ts';

const facts = {
  directory: '/work/app',
  resolved: true,
  isHome: false,
  trusted: false,
  canShowDialog: false,
};

spec('trust.behavior.Project allow rules and workspace trust', () => {
  test('a trusted directory starts; from a terminal there an untrusted one shows the dialog first', () => {
    expect(backgroundSessionTrust({ ...facts, trusted: true })).toEqual({ verdict: 'start' });
    expect(backgroundSessionTrust({ ...facts, canShowDialog: true })).toEqual({
      verdict: 'dialog',
    });
  });

  test('where no dialog can appear, the command exits with Workspace not trusted', () => {
    expect(backgroundSessionTrust(facts)).toEqual({
      verdict: 'refuse',
      message:
        'Workspace not trusted. Run `claude` in /work/app once and accept the trust prompt, then retry.',
    });
  });

  test('the home directory is never trusted from an earlier session, and a missing directory is named', () => {
    const home = { ...facts, directory: '/home/me', isHome: true, trusted: true };
    expect(backgroundSessionTrust(home)).toEqual({
      verdict: 'refuse',
      message: 'The home directory is trusted one session at a time',
    });
    expect(backgroundSessionTrust({ ...home, canShowDialog: true })).toEqual({ verdict: 'dialog' });
    expect(backgroundSessionTrust({ ...facts, resolved: false, canShowDialog: true })).toEqual({
      verdict: 'refuse',
      message: '/work/app could not be resolved on disk',
    });
  });
});
