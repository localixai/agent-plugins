//! A `try { await port.method(…) } catch (cause) { … }` around the host's values: [`catch_host`]
//! runs a future as a scope, and a throw of the host's (a port method or a callback that threw,
//! a promise that rejected) that a need's future meets while the scope polls it is caught by
//! the scope instead of ending the task (docs/RUST.md, "Effects").
//!
//! The scope that catches is the one awaiting the need when its answer is read, as in JavaScript,
//! where the `try` that awaits a promise catches its rejection, wherever the call was made. The
//! innermost scope catches; a throw outside every scope ends the task with `{"rethrow": need}`,
//! the very value the host threw. A catch that decides to throw again
//! (`if (signal?.aborted === true) throw cause`) awaits [`Thrown::rethrow`], which the scope
//! around it catches in turn, or which ends the task.
//!
//! A Rust host's ports never throw (their failures are values), so for it `catch_host` is a
//! pass-through that answers `Ok`.

use std::future::{Future, pending};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::task::with_current;

/// What the host threw for a need, as a catch reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thrown {
    /// `cause instanceof Error ? cause.message : String(cause)`.
    pub message: String,
    /// The error's `name` (`TypeError`, `AbortError`), when what it threw is an `Error`.
    pub name: Option<String>,
    /// The need the host threw for; the host keeps the thrown value by it.
    pub need: u32,
}

impl Thrown {
    /// `throw cause`: the scope around the await catches it again, or the task ends with
    /// `{"rethrow": need}`, so the call throws the very value the host threw. The future never
    /// completes, so it stands for any `T`: `return cause.rethrow().await;`.
    pub async fn rethrow<T>(self) -> T {
        raise(self);
        pending::<T>().await
    }
}

/// The host threw `thrown`, met in the task being polled: its innermost scope catches it, or the
/// task ends with it. Nothing outside a task, where no host throws.
pub(super) fn raise(thrown: Thrown) {
    with_current(|link| link.raise(thrown));
}

/// `future` run as a `try` scope: its output as `Ok`, or `Err` with the host's throw that it met
/// (then `future` is dropped, and what it was still waiting on with it).
pub fn catch_host<F: Future>(future: F) -> impl Future<Output = Result<F::Output, Thrown>> {
    CatchHost {
        future: Some(Box::pin(future)),
    }
}

/// The scope's future: its inner one until it completes or a throw ends it.
struct CatchHost<F> {
    future: Option<Pin<Box<F>>>,
}

impl<F: Future> Future for CatchHost<F> {
    type Output = Result<F::Output, Thrown>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let Some(inner) = self.future.as_mut() else {
            return Poll::Pending;
        };
        // This scope's place among the scopes being polled; `None` outside a task.
        let place = with_current(|link| {
            link.scopes.push(None);
            link.scopes.len().saturating_sub(1)
        });
        let polled = inner.as_mut().poll(context);
        let thrown = place.and_then(|place| {
            with_current(|link| {
                let caught = link.scopes.get_mut(place).and_then(Option::take);
                link.scopes.truncate(place);
                caught
            })
            .flatten()
        });
        match (polled, thrown) {
            // A future that completed did not need the answer that threw (a branch it gave up).
            (Poll::Ready(output), _) => {
                self.future = None;
                Poll::Ready(Ok(output))
            }
            (Poll::Pending, Some(thrown)) => {
                // Dropped inside the task's poll, so the needs it still waits on are retracted.
                self.future = None;
                Poll::Ready(Err(thrown))
            }
            (Poll::Pending, None) => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests;
