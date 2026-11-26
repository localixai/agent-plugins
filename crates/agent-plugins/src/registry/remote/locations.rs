//! [`Port`] as the host's locations (`src/ports/locations.ts`).

use super::Port;
use crate::js::JsValue;
use crate::ports::BoxFuture;
use crate::ports::locations::Locations;

impl Locations for Port {
    fn home_dir(&self) -> BoxFuture<'_, String> {
        self.ask("homeDir", vec![])
    }

    fn project_dir(&self) -> BoxFuture<'_, String> {
        self.ask("projectDir", vec![])
    }

    fn plugin_root<'a>(&'a self, plugin_id: &'a str) -> BoxFuture<'a, String> {
        self.ask("pluginRoot", vec![JsValue::from(plugin_id)])
    }

    fn plugin_data<'a>(&'a self, plugin_id: &'a str) -> BoxFuture<'a, String> {
        self.ask("pluginData", vec![JsValue::from(plugin_id)])
    }

    fn join_path<'a>(&'a self, base: &'a str, relative_path: &'a str) -> BoxFuture<'a, String> {
        self.ask(
            "joinPath",
            vec![JsValue::from(base), JsValue::from(relative_path)],
        )
    }

    fn existing_directory<'a>(&'a self, candidates: &'a [String]) -> BoxFuture<'a, Option<String>> {
        let candidates = candidates
            .iter()
            .map(|one| JsValue::from(one.as_str()))
            .collect();
        self.ask("existingDirectory", vec![JsValue::Array(candidates)])
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{exchange, need, port};
    use crate::js::JsValue;
    use crate::ports::locations::Locations as _;

    #[test]
    fn each_place_is_asked_by_the_typescript_name() {
        let (asked, read) = exchange(async { port().home_dir().await }, "/home/a".into());
        assert_eq!(
            (asked, read.as_str()),
            (need("homeDir", vec![]), r#""/home/a""#)
        );
        let (asked, _) = exchange(async { port().join_path("/a", "b").await }, "/a/b".into());
        assert_eq!(asked, need("joinPath", vec!["/a".into(), "b".into()]));
        let candidates = vec!["/x".to_owned(), "/y".to_owned()];
        let (asked, read) = exchange(
            async move { port().existing_directory(&candidates).await },
            JsValue::Undefined,
        );
        let list = JsValue::Array(vec!["/x".into(), "/y".into()]);
        assert_eq!(
            (asked, read.as_str()),
            (need("existingDirectory", vec![list]), "None")
        );
    }
}
