//! [`Port`] as the host's scope folders (`src/ports/folder-scan.ts`): an opened folder is a port
//! of its own, and its `dir` and `files` properties are read as the needs `.dir` and `.files`.

use std::sync::Arc;

use super::{Port, arg};
use crate::js::JsValue;
use crate::ports::files::PluginFiles;
use crate::ports::folder_scan::{FolderScan, RepositoryChain, ScanFolder};
use crate::ports::{BoxFuture, Signal};

impl ScanFolder for Port {
    fn dir(&self) -> BoxFuture<'_, String> {
        Box::pin(self.member("dir"))
    }

    fn files(&self) -> BoxFuture<'_, Arc<dyn PluginFiles>> {
        let files = self.member::<Self>("files");
        Box::pin(async move {
            let files: Arc<dyn PluginFiles> = Arc::new(files.await);
            files
        })
    }

    fn host_path<'a>(&'a self, relative_path: &'a str) -> BoxFuture<'a, String> {
        self.ask("hostPath", vec![JsValue::from(relative_path)])
    }

    fn canonical_path<'a>(&'a self, relative_path: &'a str) -> BoxFuture<'a, Option<String>> {
        self.ask("canonicalPath", vec![JsValue::from(relative_path)])
    }
}

impl FolderScan for Port {
    fn managed_directory(&self) -> BoxFuture<'_, Option<String>> {
        self.ask("managedDirectory", vec![])
    }

    fn open<'a>(
        &'a self,
        dir: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<Arc<dyn ScanFolder>>> {
        let opened = self.ask::<Option<Self>>("open", vec![JsValue::from(dir), arg(signal)]);
        Box::pin(async move {
            opened.await.map(|folder| {
                let folder: Arc<dyn ScanFolder> = Arc::new(folder);
                folder
            })
        })
    }

    fn repository<'a>(
        &'a self,
        cwd: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, RepositoryChain> {
        self.at("repository", cwd, signal)
    }

    fn directories_below<'a>(
        &'a self,
        start: &'a str,
        touched: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Vec<String>> {
        self.ask(
            "directoriesBelow",
            vec![JsValue::from(start), JsValue::from(touched), arg(signal)],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{exchange, json, need, port};
    use crate::js::JsValue;
    use crate::ports::folder_scan::FolderScan as _;

    #[test]
    fn an_opened_folder_is_a_port_whose_properties_are_asked() {
        let (asked, read) = exchange(
            async {
                let folder = port().open("/p/.claude", None).await;
                folder.is_some()
            },
            json(r#"{"$port": 9}"#),
        );
        assert_eq!(
            (asked, read.as_str()),
            (
                need("open", vec!["/p/.claude".into(), JsValue::Undefined]),
                "true"
            )
        );
        let (asked, read) = exchange(
            async { port().repository("/p/a", None).await },
            json(r#"{"directories": ["/p/a", "/p"]}"#),
        );
        assert_eq!(
            asked,
            need("repository", vec!["/p/a".into(), JsValue::Undefined])
        );
        assert_eq!(
            read,
            r#"RepositoryChain { directories: ["/p/a", "/p"], main_checkout: None }"#
        );
    }

    #[test]
    fn a_folders_dir_is_read_as_a_property() {
        use crate::ports::folder_scan::ScanFolder as _;
        let (asked, read) = exchange(async { port().dir().await }, "/p".into());
        assert_eq!((asked, read.as_str()), (need(".dir", vec![]), r#""/p""#));
    }
}
