/**
 * errors.md, "Path escapes plugin directory" (the `claude plugin` form) and "Path could not be
 * checked" (ESTALE reported as EIO, J17.29).
 */
import { expect, test } from 'bun:test';
import {
  pathCouldNotBeChecked,
  pathCouldNotBeCheckedInList,
  pathEscapesInList,
} from '../../src/format/host-errors.ts';
import { spec } from '../support/spec.ts';

const E = 'cli.errors.';

spec(`${E}Path escapes plugin directory`, () => {
  test('claude plugin commands print the path, then the component in parentheses', () => {
    expect(pathEscapesInList('commands', './../shared.md')).toBe(
      'Path escapes plugin directory: ./../shared.md (commands)',
    );
  });
});

// A judgment of the specification (J17.29: errors.md names "EIO or ESTALE" together), so outside spec().
test('ESTALE is reported as EIO', () => {
  expect(pathCouldNotBeChecked('skills', '/p/skills', 'ESTALE')).toBe(
    "skills path couldn't be checked: /p/skills (EIO)",
  );
  expect(pathCouldNotBeCheckedInList('skills', '/p/skills', 'ESTALE')).toBe(
    'Path not found: /p/skills (skills, EIO)',
  );
});
