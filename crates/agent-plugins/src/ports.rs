//! Layer 1: the ports, the traits a host implements for every effect the crate needs
//! (`src/ports/` in the npm package). One file per port.
//!
//! The npm package's ports return promises; here a port method returns a [`BoxFuture`], a boxed
//! future that names no runtime. A host on an async runtime runs the crate's futures on it, and
//! hooks that the docs run in parallel run in parallel; a host with none drives them with any
//! `block_on`, and its ports may simply block. The crate itself never spawns, sleeps or blocks.
//! An `AbortSignal` of the npm package is a [`Signal`]. A host whose ports answer at once (a
//! command-line tool with no runtime, reading files as it is asked) returns ready futures and
//! takes each of the crate's answers with [`now`].

pub mod clock;
pub mod command_runner;
pub mod environment;
pub mod failure;
pub mod files;
pub mod folder_scan;
pub mod http_client;
pub mod locations;
pub mod marketplace;
pub mod mcp_bundle;
pub mod mcp_caller;
pub mod model_runner;
pub mod path_resolver;
pub mod scratch_files;
pub mod signal;

use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll, Waker};

pub use failure::PortFailure;
pub use signal::Signal;

/// The future a port method returns: boxed so the traits stay object safe, `Send` so a host on a
/// multi-threaded runtime can run it anywhere.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The future's output when it is ready at its first poll, `None` when it waits. Every future of
/// the crate is ready at once when every port it calls answers with a ready future
/// (`Box::pin(std::future::ready(answer))`), so a host with no async runtime needs nothing more;
/// it never blocks (RS3).
pub fn now<F: Future>(future: F) -> Option<F::Output> {
    match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(output) => Some(output),
        Poll::Pending => None,
    }
}

#[cfg(test)]
mod tests {
    use std::future::ready;

    use super::files::{DirectoryEntry, FileStat, PluginFiles, TextFileReader};
    use super::{BoxFuture, Signal, now};
    use crate::format::read_json::{JsonRead, read_json};

    /// A host's files answered at once, as a host with no runtime writes them.
    struct OneFile;

    impl TextFileReader for OneFile {
        fn read_text<'a>(
            &'a self,
            path: &'a str,
            _: Option<&'a Signal>,
        ) -> BoxFuture<'a, Option<String>> {
            Box::pin(ready(
                (path == "plugin.json").then(|| r#"{"name":"a"}"#.to_owned()),
            ))
        }
    }

    impl PluginFiles for OneFile {
        fn read_bytes<'a>(
            &'a self,
            _: &'a str,
            _: Option<&'a Signal>,
        ) -> BoxFuture<'a, Option<Vec<u8>>> {
            Box::pin(ready(None))
        }
        fn list<'a>(
            &'a self,
            _: &'a str,
            _: Option<&'a Signal>,
        ) -> BoxFuture<'a, Option<Vec<DirectoryEntry>>> {
            Box::pin(ready(None))
        }
        fn stat<'a>(
            &'a self,
            _: &'a str,
            _: Option<&'a Signal>,
        ) -> BoxFuture<'a, Option<FileStat>> {
            Box::pin(ready(None))
        }
        fn exists<'a>(&'a self, path: &'a str, _: Option<&'a Signal>) -> BoxFuture<'a, bool> {
            Box::pin(ready(path == "plugin.json"))
        }
    }

    #[test]
    fn ports_that_answer_at_once_give_every_answer_at_the_first_poll() {
        let read = now(read_json(&OneFile, "plugin.json", None));
        assert!(matches!(read, Some(JsonRead::Value { .. })));
        let missing = now(read_json(&OneFile, "absent.json", None));
        assert!(matches!(
            missing,
            Some(JsonRead::Problem { missing: true, .. })
        ));
        assert_eq!(now(std::future::pending::<()>()), None);
    }
}
