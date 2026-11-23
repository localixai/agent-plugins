/**
 * Agent Plugins Extended §5.12.4 (the `web-fetch` row of the built-in table) and §7.6.3 item 8: a
 * session that reads web pages through the built-in `web-fetch` agent (signed in with a claude.ai
 * account, feature flags fetched, no LLM gateway: what the host knows, `SessionOptions.webFetchAgent`)
 * registers it; the deny rule `Agent(web-fetch)` turns it off; a session that does not, has none.
 */
import { describe, expect, test } from 'bun:test';
import { namesOf } from '../support/agent-host.ts';
import { folderHostOf } from '../support/scan.ts';
import { parsed } from '../support/rule-context.ts';
import { spec } from '../support/spec.ts';
import { tierOf } from '../support/agent-catalog.ts';

spec('agents.behavior.Built-in subagents', () => {
  describe('the built-in web-fetch agent in a session', () => {
    test('is registered when the session reads web pages through it', () => {
      const { session } = folderHostOf({}, { session: { webFetchAgent: true } });
      const found = session.agentCatalog().agents.find((agent) => agent.name === 'web-fetch');
      expect(tierOf(found)).toBe('builtin');
      expect(found?.definition.tools).toEqual(['WebFetch']);
    });

    test('is not registered when the session does not', () => {
      const { session } = folderHostOf({});
      expect(namesOf(session)).not.toContain('web-fetch');
      expect(namesOf(session)).toContain('general-purpose');
    });

    test('the deny rule Agent(web-fetch) turns it off', () => {
      const { session } = folderHostOf(
        {},
        { session: { webFetchAgent: true, permissionRules: [parsed('Agent(web-fetch)')] } },
      );
      expect(namesOf(session)).not.toContain('web-fetch');
    });

    test('a run that says otherwise decides for that call (AgentOverrides.webFetchAgent)', () => {
      const { session } = folderHostOf({}, { session: { webFetchAgent: true } });
      const names = (webFetchAgent: boolean) =>
        session.agentCatalog({ webFetchAgent }).agents.map((agent) => agent.name);
      expect(names(false)).not.toContain('web-fetch');
      expect(names(true)).toContain('web-fetch');
    });
  });
});
