/**
 * The unquoted-placeholder warning of `claude plugin validate` (oracle 2.1.325, spec §3.2.3 item 5
 * and J3.20): the placeholders are named in the order they first appear in the command, each once.
 * Tested outside spec(): the wording and order are the oracle's.
 */
import { expect, test } from 'bun:test';
import { unquotedMessage } from '../../../src/format/validate/shell-command.ts';

test('the placeholders are listed in order of first appearance, each once', () => {
  const command = 'cd ${CLAUDE_PROJECT_DIR} && ${CLAUDE_PLUGIN_ROOT}/run $CLAUDE_PROJECT_DIR';
  expect(unquotedMessage(command)).toStartWith(
    `Shell command uses \${CLAUDE_PROJECT_DIR}, \${CLAUDE_PLUGIN_ROOT} without quotes: ${command}. `,
  );
});

test('a quoted use is not listed', () => {
  expect(unquotedMessage('"${CLAUDE_PLUGIN_DATA}" && ${CLAUDE_PLUGIN_ROOT}/x')).toStartWith(
    'Shell command uses ${CLAUDE_PLUGIN_ROOT} without quotes',
  );
  expect(unquotedMessage("'${CLAUDE_PLUGIN_ROOT}'/x")).toBeUndefined();
});
