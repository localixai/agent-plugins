//! [`Port`] as the host's `realpath` (`src/ports/path-resolver.ts`): `string | PortFailure |
//! undefined`, read as `Option<Result<…>>`.

use super::{Failable, Port};
use crate::js::JsValue;
use crate::ports::path_resolver::PathResolver;
use crate::ports::{BoxFuture, PortFailure};

impl PathResolver for Port {
    fn realpath<'a>(&'a self, path: &'a str) -> BoxFuture<'a, Option<Result<String, PortFailure>>> {
        let asked = self.ask::<Option<Failable<String>>>("realpath", vec![JsValue::from(path)]);
        Box::pin(async move { asked.await.map(Failable::into_result) })
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{exchange, json, need, port};
    use crate::js::JsValue;
    use crate::ports::path_resolver::PathResolver as _;

    #[test]
    fn a_path_a_failure_and_none_are_read_back() {
        let (asked, read) = exchange(async { port().realpath("/a").await }, "/b".into());
        assert_eq!(
            (asked, read.as_str()),
            (need("realpath", vec!["/a".into()]), r#"Some(Ok("/b"))"#)
        );
        let failure = json(r#"{"failure": "loop", "message": "ELOOP"}"#);
        let (_, read) = exchange(async { port().realpath("/a").await }, failure);
        assert_eq!(
            read,
            r#"Some(Err(PortFailure { failure: "loop", message: "ELOOP" }))"#
        );
        let (_, read) = exchange(async { port().realpath("/a").await }, JsValue::Undefined);
        assert_eq!(read, "None");
    }
}
