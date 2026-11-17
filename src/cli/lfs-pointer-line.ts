/**
 * The line `marketplace add` and `marketplace update` print after a git clone that left LFS-tracked
 * files as pointer files (spec §14.8.4, the `git` source row: "the add or update output reports how
 * many"; plugins/host-marketplace.md, "Keep plugin files out of Git LFS"). The host counts them
 * (`lfsPointerIssues` finds them file by file). Judgment: the wording (spec/judgments.md).
 */

/** The line for `count` pointer files, or none when the clone left none. */
export function lfsPointerLine(count: number): string | undefined {
  if (count <= 0) return undefined;
  const files = count === 1 ? 'file arrived as a pointer file' : 'files arrived as pointer files';
  return `${count} Git LFS ${files}: the clone never downloads LFS content`;
}
