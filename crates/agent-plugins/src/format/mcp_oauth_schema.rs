//! Port of `src/format/mcp-oauth-schema.ts` at sha256:b1605ba7a573.
//!
//! The types of a remote server's `oauth` block as the entry is checked (spec §8.2 and §8.7.2, J8.3):
//! a wrong type, or an `authServerMetadataUrl` that is not `https://` ("The URL must use `https://`",
//! mcp.md, "Override OAuth metadata discovery"), makes the entry invalid, so the loader drops it and
//! the validator reports it (`oauth.scopes: Invalid input: expected string, received array`). An
//! unknown key inside is kept and ignored; `readMcpOAuth` reads the block's values.

use std::sync::LazyLock;

use crate::js::JsValue;
use crate::js::url::parse;
use crate::zod::{self, Schema, fields};

fn is_https(text: &JsValue) -> bool {
    text.as_str()
        .and_then(parse)
        .is_some_and(|url| url.protocol == "https:")
}

const HTTPS_METADATA: &str = "authServerMetadataUrl must use https://";

/// `mcpOAuthSchema`: the `oauth` block, its unknown keys kept.
pub static MCP_OAUTH_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    zod::loose_object(fields! {
        "clientId" => zod::string().optional(),
        "callbackPort" => zod::number().optional(),
        "authServerMetadataUrl" => zod::string().refine(is_https, HTTPS_METADATA).optional(),
        "scopes" => zod::string().optional(),
    })
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::json_parse::json_parse;

    #[test]
    fn a_metadata_url_must_be_https() {
        let block = |text: &str| json_parse(text).unwrap_or(JsValue::Undefined);
        let refused = MCP_OAUTH_SCHEMA.safe_parse(&block(
            r#"{"authServerMetadataUrl":"http://a.example/x","other":1}"#,
        ));
        let messages: Vec<String> = refused
            .err()
            .map(|failure| {
                failure
                    .issues
                    .iter()
                    .map(|issue| issue.message().to_owned())
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(messages, [HTTPS_METADATA]);
        let kept = MCP_OAUTH_SCHEMA.safe_parse(&block(
            r#"{"authServerMetadataUrl":"https://a.example/x","other":1}"#,
        ));
        assert!(kept.is_ok_and(|value| value.get("other").is_some()));
    }
}
