//! [`Port`] as the host's scratch files (`src/ports/scratch-files.ts`).

use super::Port;
use crate::js::JsValue;
use crate::ports::BoxFuture;
use crate::ports::scratch_files::ScratchFiles;

impl ScratchFiles for Port {
    fn write<'a>(&'a self, name: &'a str, content: &'a str) -> BoxFuture<'a, String> {
        self.ask("write", vec![JsValue::from(name), JsValue::from(content)])
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{exchange, need, port};
    use crate::ports::scratch_files::ScratchFiles as _;

    #[test]
    fn a_write_answers_the_hosts_path() {
        let (asked, read) = exchange(
            async { port().write("out.txt", "long").await },
            "/tmp/out.txt".into(),
        );
        assert_eq!(asked, need("write", vec!["out.txt".into(), "long".into()]));
        assert_eq!(read, r#""/tmp/out.txt""#);
    }
}
