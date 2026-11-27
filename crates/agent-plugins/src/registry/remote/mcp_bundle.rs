//! [`Port`] as the host's bundle extractor (`src/ports/mcp-bundle.ts`): `McpBundle | PortFailure |
//! undefined`, read as `Option<Result<…>>`.

use super::{Failable, Port, arg};
use crate::js::JsValue;
use crate::ports::mcp_bundle::{McpBundle, McpBundleReader, McpBundleSource};
use crate::ports::{BoxFuture, PortFailure, Signal};

impl McpBundleReader for Port {
    fn open<'a>(
        &'a self,
        source: &'a McpBundleSource,
        cache_dir: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<Result<McpBundle, PortFailure>>> {
        let asked = self.ask::<Option<Failable<McpBundle>>>(
            "open",
            vec![arg(Some(source)), JsValue::from(cache_dir), arg(signal)],
        );
        Box::pin(async move { asked.await.map(Failable::into_result) })
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{exchange, json, need, port};
    use crate::js::JsValue;
    use crate::ports::mcp_bundle::{McpBundleReader as _, McpBundleSource};

    #[test]
    fn a_bundle_a_failure_and_none_are_read_back() {
        let source = || McpBundleSource::Path {
            path: "a.mcpb".to_owned(),
        };
        let open = |source: McpBundleSource| async move { port().open(&source, ".c", None).await };
        let opened = json(r#"{"name": "s", "server": {"command": "x"}}"#);
        let (asked, read) = exchange(open(source()), opened);
        assert_eq!(
            asked,
            need(
                "open",
                vec![
                    json(r#"{"kind": "path", "path": "a.mcpb"}"#),
                    ".c".into(),
                    JsValue::Undefined
                ]
            )
        );
        assert!(
            read.starts_with(r#"Some(Ok(McpBundle { name: "s""#),
            "{read}"
        );
        let failure = json(r#"{"failure": "io", "message": "EIO"}"#);
        let (_, read) = exchange(open(source()), failure);
        assert!(read.starts_with("Some(Err("), "{read}");
        let (_, read) = exchange(open(source()), JsValue::Undefined);
        assert_eq!(read, "None");
    }
}
