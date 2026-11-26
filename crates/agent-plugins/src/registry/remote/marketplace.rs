//! [`Port`] as what installing from a marketplace needs of the host (`src/ports/marketplace.ts`):
//! `Sha256`, `ArchiveReader`, `GitTags` and `PluginStateStore`. Bytes cross as the wire tags them;
//! `undefined | StateStoreFailure` is `Ok(())` or the failure.

use super::{Failable, Port, arg};
use crate::js::JsValue;
use crate::ports::marketplace::{
    ArchiveReader, ArchiveScan, GitRepository, GitTags, PluginStateStore, Sha256,
};
use crate::ports::{BoxFuture, PortFailure, Signal};
use crate::registry::wire::Bytes;

fn bytes_arg(bytes: &[u8]) -> JsValue {
    arg(Some(&Bytes(bytes.to_vec())))
}

impl Sha256 for Port {
    fn hex<'a>(&'a self, bytes: &'a [u8]) -> BoxFuture<'a, String> {
        self.ask("hex", vec![bytes_arg(bytes)])
    }
}

impl ArchiveReader for Port {
    fn scan<'a>(
        &'a self,
        bytes: &'a [u8],
        signal: &'a Signal,
    ) -> BoxFuture<'a, Result<ArchiveScan, PortFailure>> {
        self.ask_failable("scan", vec![bytes_arg(bytes), arg(Some(signal))])
    }
}

impl GitTags for Port {
    fn list<'a>(
        &'a self,
        repository: &'a GitRepository,
        signal: &'a Signal,
    ) -> BoxFuture<'a, Result<Vec<String>, PortFailure>> {
        self.ask_failable("list", vec![arg(Some(repository)), arg(Some(signal))])
    }
}

impl Port {
    /// A store's change, answered `undefined | StateStoreFailure`.
    fn changed(
        self,
        method: &'static str,
        args: Vec<JsValue>,
    ) -> BoxFuture<'static, Result<(), PortFailure>> {
        let asked = self.ask::<Option<Failable<JsValue>>>(method, args);
        Box::pin(async move {
            match asked.await {
                Some(Failable::Failure(failure)) => Err(failure),
                Some(Failable::Value(_)) | None => Ok(()),
            }
        })
    }
}

impl PluginStateStore for Port {
    fn read<'a>(&'a self, name: &'a str, signal: &'a Signal) -> BoxFuture<'a, Option<String>> {
        self.ask("read", vec![JsValue::from(name), arg(Some(signal))])
    }

    fn write<'a>(
        &'a self,
        name: &'a str,
        text: &'a str,
        signal: &'a Signal,
    ) -> BoxFuture<'a, Result<(), PortFailure>> {
        let args = vec![JsValue::from(name), JsValue::from(text), arg(Some(signal))];
        self.changed("write", args)
    }

    fn names<'a>(&'a self, signal: &'a Signal) -> BoxFuture<'a, Vec<String>> {
        self.ask("names", vec![arg(Some(signal))])
    }

    fn modified_at<'a>(&'a self, name: &'a str, signal: &'a Signal) -> BoxFuture<'a, Option<f64>> {
        self.ask("modifiedAt", vec![JsValue::from(name), arg(Some(signal))])
    }

    fn remove<'a>(
        &'a self,
        name: &'a str,
        signal: &'a Signal,
    ) -> BoxFuture<'a, Result<(), PortFailure>> {
        self.changed("remove", vec![JsValue::from(name), arg(Some(signal))])
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{exchange, json, need, port};
    use crate::js::JsValue;
    use crate::ports::Signal;
    use crate::ports::marketplace::{ArchiveReader as _, GitRepository, GitTags as _};
    use crate::ports::marketplace::{PluginStateStore as _, Sha256 as _};

    fn signal() -> JsValue {
        json(r#"{"$signal": null, "aborted": false}"#)
    }

    #[test]
    fn bytes_cross_tagged_and_a_digest_is_read_back() {
        let (asked, read) = exchange(async { port().hex(&[0, 1, 255]).await }, "ab".into());
        assert_eq!(asked, need("hex", vec![json(r#"{"$bytes": "AAH/"}"#)]));
        assert_eq!(read, r#""ab""#);
        let signal_value = Signal::new();
        let (asked, read) = exchange(
            async move { port().scan(&[1], &signal_value).await },
            json(r#"{"failure": "corrupt", "message": "bad"}"#),
        );
        assert_eq!(
            asked,
            need("scan", vec![json(r#"{"$bytes": "AQ=="}"#), signal()])
        );
        assert_eq!(
            read,
            r#"Err(PortFailure { failure: "corrupt", message: "bad" })"#
        );
    }

    #[test]
    fn tags_and_the_store_answer_as_the_typescript_types_them() {
        let repository = GitRepository::Shorthand {
            shorthand: "a/b".to_owned(),
        };
        let (asked, read) = exchange(
            async move { port().list(&repository, &Signal::new()).await },
            json(r#"["v1"]"#),
        );
        assert_eq!(
            asked,
            need("list", vec![json(r#"{"shorthand": "a/b"}"#), signal()])
        );
        assert_eq!(read, r#"Ok(["v1"])"#);
        let (asked, read) = exchange(
            async { port().write("a.json", "{}", &Signal::new()).await },
            JsValue::Undefined,
        );
        assert_eq!(
            asked,
            need("write", vec!["a.json".into(), "{}".into(), signal()])
        );
        assert_eq!(read, "Ok(())");
        let failure = json(r#"{"failure": "remove", "message": "EBUSY"}"#);
        let (_, read) = exchange(async { port().remove("a", &Signal::new()).await }, failure);
        assert_eq!(
            read,
            r#"Err(PortFailure { failure: "remove", message: "EBUSY" })"#
        );
    }
}
