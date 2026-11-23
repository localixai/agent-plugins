/**
 * Where `CLAUDE_CODE_EXPERIMENTAL_CODE_MODE` takes effect (spec §12.7.4 item 3): "turns Code Mode on
 * only from the launch environment, user settings or `--settings`, never from managed, project or
 * local settings; a `0` in any of those three or in managed settings keeps Code Mode off whatever
 * the others say, and under a managed `1` a `1` in the launch environment is ignored, so the user's
 * own `1` goes in user settings." Project and local values never reach here: the repository drops
 * them (`ignoredBecause`, reason `code-mode`). Judgment (spec/judgments.md): `1` turns it on and `0`
 * off; any other value is neither.
 */
export interface CodeModeValues {
  /** The launch environment's value. */
  readonly launch?: string | undefined;
  /** The `env` value of user settings. */
  readonly user?: string | undefined;
  /** The `env` value of `--settings`. */
  readonly flag?: string | undefined;
  /** The `env` value of managed settings. */
  readonly managed?: string | undefined;
}

export function codeModeOn(values: CodeModeValues): boolean {
  const { launch, user, flag, managed } = values;
  if ([launch, user, flag, managed].includes('0')) return false;
  const launchCounts = launch === '1' && managed !== '1';
  return user === '1' || flag === '1' || launchCounts;
}
