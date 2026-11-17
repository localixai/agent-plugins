/** The flags of `claude plugin uninstall` that bear on whether it can run. */
interface UninstallFlags {
  readonly prune?: boolean;
  readonly yes?: boolean;
  readonly json?: boolean;
}

/** `--json` and a missing `-y` are flag problems: nothing is printed but the reason. */
export function flagProblem(args: UninstallFlags, tty: boolean): string | undefined {
  if (args.json === true && args.prune === true) return '--json cannot be combined with --prune';
  if (args.prune === true && args.yes !== true && !tty) {
    return '--prune needs -y when stdin or stdout is not a terminal';
  }
  return undefined;
}
