/**
 * The refusals of `permissions.blockWritesOutsideWorkingDirectories`, verbatim (errors.md, "Setting
 * blocks writes outside the working directories" and "Writes to Claude Code settings files are
 * blocked"; settings-reference.md). `decide` answers with the reason (`outside-working-directory`,
 * `settings-file`); the host shows Claude the text the tool call fails with.
 */

/** What a write the block refuses is refused for. */
export type WriteBlockReason = 'outside-working-directory' | 'settings-file';

/** The facts the texts name: the path written, and the working directories it is outside of. */
export interface WriteBlockRefusalInput {
  readonly path: string;
  readonly workingDirectories: readonly string[];
}

const REFUSALS: {
  readonly [R in WriteBlockReason]: (input: WriteBlockRefusalInput) => string;
} = {
  'outside-working-directory': ({ path, workingDirectories }) =>
    `${path} is outside ${workingDirectories.join(', ')}; the permissions.blockWritesOutsideWorkingDirectories setting blocks writes outside the working directories. Ask the user to add the directory with /add-dir, or to remove that setting.`,
  'settings-file': ({ path }) =>
    `${path}: Writes to Claude Code settings files are blocked while permissions.blockWritesOutsideWorkingDirectories is on, because a settings file can lift that block. While a managed settings folder cannot be resolved, any .json file counts as one.`,
};

/**
 * The text a write the block refuses fails with. The working directories are joined with `, `
 * (judgment, spec/judgments.md: the docs show the placeholder `<dirs>` only).
 */
export const writeBlockRefusal = (
  reason: WriteBlockReason,
  input: WriteBlockRefusalInput,
): string => REFUSALS[reason](input);

/** "The refusal can also read" this, for a write outside the working directories. */
export const writeBlockShortRefusal = (path: string): string =>
  `${path}: Writes outside the working directories are blocked (permissions.blockWritesOutsideWorkingDirectories). Add the directory with /add-dir, or remove that setting.`;
