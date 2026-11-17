/**
 * server-managed-settings.md § Enforce fail-closed startup ("If the gateway answers an attended
 * interactive launch with a `401` and this setting is off ... prints `Cloud gateway session expired
 * — run /login to reconnect.` and opens the session signed out of the gateway"); errors.md § Cloud
 * gateway session expired. Spec 1.1.0 §14.3.3 item 3.
 */
import { expect, test } from 'bun:test';
import { startupFetchFailure } from '../../../src/settings/managed/index.ts';
import { spec } from '../../support/spec.ts';

const gateway = {
  forceRemoteSettingsRefresh: false,
  fetchesRemote: true,
  signsInAtStartup: false,
  anthropicRequires: false,
  gatewaySession: true,
  authSubcommand: false,
  attendedInteractive: true,
};

spec('policy.server.Enforce fail-closed startup', () => {
  test('a gateway 401 on an attended interactive launch prints the line and opens signed out', () => {
    expect(startupFetchFailure({ ...gateway, gatewayStatus: 401 })).toEqual({
      outcome: 'opens-signed-out',
      message: 'Cloud gateway session expired — run /login to reconnect.',
    });
  });

  test('with forceRemoteSettingsRefresh on, the same 401 exits', () => {
    expect(
      startupFetchFailure({ ...gateway, gatewayStatus: 401, forceRemoteSettingsRefresh: true }),
    ).toEqual({ outcome: 'exits' });
  });

  test('any other failure, or any other launch, exits', () => {
    expect(startupFetchFailure({ ...gateway, gatewayStatus: 500 })).toEqual({ outcome: 'exits' });
    expect(startupFetchFailure(gateway)).toEqual({ outcome: 'exits' });
    expect(
      startupFetchFailure({ ...gateway, gatewayStatus: 401, attendedInteractive: false }),
    ).toEqual({ outcome: 'exits' });
  });

  test('a claude auth subcommand is exempt', () => {
    expect(startupFetchFailure({ ...gateway, authSubcommand: true })).toEqual({
      outcome: 'starts',
    });
  });

  test('outside a gateway, the key or Anthropic requiring the settings exits; otherwise it starts', () => {
    const direct = { ...gateway, gatewaySession: false };
    expect(startupFetchFailure(direct)).toEqual({ outcome: 'starts' });
    expect(startupFetchFailure({ ...direct, forceRemoteSettingsRefresh: true })).toEqual({
      outcome: 'exits',
    });
    expect(startupFetchFailure({ ...direct, anthropicRequires: true })).toEqual({
      outcome: 'exits',
    });
  });
});
