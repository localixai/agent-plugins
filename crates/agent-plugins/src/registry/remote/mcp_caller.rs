//! [`Port`] as the host's MCP connections (`src/ports/mcp-caller.ts`): `McpToolResult |
//! PortFailure`, read as a `Result`.

use super::{Port, arg};
use crate::js::JsValue;
use crate::ports::mcp_caller::{McpAvailability, McpCaller, McpToolCall, McpToolResult};
use crate::ports::{BoxFuture, PortFailure, Signal};

impl McpCaller for Port {
    fn ensure_connected<'a>(
        &'a self,
        server: &'a str,
        wait_ms: u64,
        signal: &'a Signal,
    ) -> BoxFuture<'a, McpAvailability> {
        let args = vec![
            JsValue::from(server),
            arg(Some(&wait_ms)),
            arg(Some(signal)),
        ];
        self.ask("ensureConnected", args)
    }

    fn call_tool(&self, call: McpToolCall) -> BoxFuture<'_, Result<McpToolResult, PortFailure>> {
        self.ask_failable("callTool", vec![arg(Some(&call))])
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{exchange, json, need, port};
    use crate::js::JsObject;
    use crate::ports::Signal;
    use crate::ports::mcp_caller::{McpCaller as _, McpToolCall};

    #[test]
    fn a_connection_and_a_tool_call_are_asked() {
        let (asked, read) = exchange(
            async { port().ensure_connected("db", 100, &Signal::new()).await },
            "connected".into(),
        );
        let signal = json(r#"{"$signal": null, "aborted": false}"#);
        assert_eq!(
            asked,
            need("ensureConnected", vec!["db".into(), 100.0.into(), signal])
        );
        assert_eq!(read, "Connected");
        let call = McpToolCall {
            server: "db".to_owned(),
            tool: "query".to_owned(),
            input: JsObject::new(),
            timeout_ms: None,
            signal: Signal::new(),
        };
        let (asked, read) = exchange(
            async move { port().call_tool(call).await },
            json(r#"{"isError": false, "text": "1 row"}"#),
        );
        let sent = json(
            r#"{"server": "db", "tool": "query", "input": {},
            "signal": {"$signal": null, "aborted": false}}"#,
        );
        assert_eq!(asked, need("callTool", vec![sent]));
        assert_eq!(
            read,
            r#"Ok(McpToolResult { is_error: false, text: "1 row", content: None, structured_content: None })"#
        );
    }
}
