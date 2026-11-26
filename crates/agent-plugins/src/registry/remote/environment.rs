//! [`Port`] as the host's environment (`src/ports/environment.ts`).

use indexmap::IndexMap;

use super::Port;
use crate::js::JsValue;
use crate::ports::BoxFuture;
use crate::ports::environment::Environment;

impl Environment for Port {
    fn get<'a>(&'a self, name: &'a str) -> BoxFuture<'a, Option<String>> {
        self.ask("get", vec![JsValue::from(name)])
    }

    fn snapshot(&self) -> BoxFuture<'_, IndexMap<String, String>> {
        self.ask("snapshot", vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{exchange, json, need, port};
    use crate::js::JsValue;
    use crate::ports::environment::Environment as _;

    #[test]
    fn a_variable_and_the_snapshot_are_asked() {
        let (asked, read) = exchange(async { port().get("HOME").await }, JsValue::Undefined);
        assert_eq!(
            (asked, read.as_str()),
            (need("get", vec!["HOME".into()]), "None")
        );
        let (asked, read) = exchange(
            async { port().snapshot().await },
            json(r#"{"B": "2", "A": "1"}"#),
        );
        assert_eq!(
            (asked, read.as_str()),
            (need("snapshot", vec![]), r#"{"B": "2", "A": "1"}"#)
        );
    }
}
