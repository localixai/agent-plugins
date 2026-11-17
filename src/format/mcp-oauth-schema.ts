import { z } from 'zod';

/**
 * The types of a remote server's `oauth` block as the entry is checked (spec §8.2 and §8.7.2, J8.3):
 * a wrong type, or an `authServerMetadataUrl` that is not `https://` ("The URL must use `https://`",
 * mcp.md, "Override OAuth metadata discovery"), makes the entry invalid, so the loader drops it and
 * the validator reports it (`oauth.scopes: Invalid input: expected string, received array`). An
 * unknown key inside is kept and ignored; `readMcpOAuth` reads the block's values.
 */
const isHttps = (text: string): boolean =>
  URL.canParse(text) && new URL(text).protocol === 'https:';

const HTTPS_METADATA = 'authServerMetadataUrl must use https://';

export const mcpOAuthSchema = z.looseObject({
  clientId: z.string().optional(),
  callbackPort: z.number().optional(),
  authServerMetadataUrl: z.string().refine(isHttps, { error: HTTPS_METADATA }).optional(),
  scopes: z.string().optional(),
});
