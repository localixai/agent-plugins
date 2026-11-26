//! [`Port`] as the host's files (`src/ports/files.ts`): `TextFileReader` and `PluginFiles`.

use super::Port;
use crate::ports::files::{DirectoryEntry, FileStat, PluginFiles, TextFileReader};
use crate::ports::{BoxFuture, Signal};
use crate::registry::wire::Bytes;

impl TextFileReader for Port {
    fn read_text<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<String>> {
        self.at("readText", path, signal)
    }
}

impl PluginFiles for Port {
    fn read_bytes<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<Vec<u8>>> {
        let bytes = self.at::<Option<Bytes>>("readBytes", path, signal);
        Box::pin(async move { bytes.await.map(|Bytes(bytes)| bytes) })
    }

    fn list<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<Vec<DirectoryEntry>>> {
        self.at("list", path, signal)
    }

    fn stat<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<FileStat>> {
        self.at("stat", path, signal)
    }

    fn exists<'a>(&'a self, path: &'a str, signal: Option<&'a Signal>) -> BoxFuture<'a, bool> {
        self.at("exists", path, signal)
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{exchange, json, need, port};
    use crate::js::JsValue;
    use crate::ports::files::{PluginFiles as _, TextFileReader as _};

    #[test]
    fn a_text_is_asked_by_path_and_read_back() {
        let (asked, read) = exchange(
            async { port().read_text("a.md", None).await },
            "text".into(),
        );
        assert_eq!(
            asked,
            need("readText", vec!["a.md".into(), JsValue::Undefined])
        );
        assert_eq!(read, r#"Some("text")"#);
    }

    #[test]
    fn bytes_and_a_listing_are_read_back() {
        let (asked, read) = exchange(
            async { port().read_bytes("a.bin", None).await },
            json(r#"{"$bytes": "AAH/"}"#),
        );
        assert_eq!(asked.method, "readBytes");
        assert_eq!(read, "Some([0, 1, 255])");
        let (_, read) = exchange(
            async { port().list(".", None).await },
            json(r#"[{"name": "a", "isFile": true, "isDirectory": false}]"#),
        );
        assert_eq!(
            read,
            r#"Some([DirectoryEntry { name: "a", is_file: true, is_directory: false }])"#
        );
    }
}
