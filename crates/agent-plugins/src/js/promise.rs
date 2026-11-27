//! JavaScript's `Promise.all`: the futures run together, so a port that is the host's is asked
//! for all of their needs in one step (docs/RUST.md, "Effects").

use std::sync::{Arc, Mutex, PoisonError};
use std::task::Poll;

use crate::ports::BoxFuture;

/// `Promise.all(futures)`: every future is polled at each wake until all are done, and the
/// outputs come back in the order of the futures.
pub fn promise_all<'a, T: Send + 'a>(futures: Vec<BoxFuture<'a, T>>) -> BoxFuture<'a, Vec<T>> {
    let mut pending: Vec<Option<BoxFuture<'a, T>>> = futures.into_iter().map(Some).collect();
    let mut outputs: Vec<Option<T>> = pending.iter().map(|_| None).collect();
    Box::pin(std::future::poll_fn(move |context| {
        let mut waiting = false;
        for (slot, output) in pending.iter_mut().zip(outputs.iter_mut()) {
            let Some(future) = slot.as_mut() else {
                continue;
            };
            match future.as_mut().poll(context) {
                Poll::Ready(value) => {
                    *output = Some(value);
                    *slot = None;
                }
                Poll::Pending => waiting = true,
            }
        }
        if waiting {
            Poll::Pending
        } else {
            Poll::Ready(outputs.iter_mut().filter_map(Option::take).collect())
        }
    }))
}

/// `Promise.all([first, second])` of two futures of different types: both run together.
pub fn promise_all_pair<'a, A: Send + 'a, B: Send + 'a>(
    first: BoxFuture<'a, A>,
    second: BoxFuture<'a, B>,
) -> BoxFuture<'a, (A, B)> {
    let (mut first, mut second) = (first, second);
    let (mut first_output, mut second_output): (Option<A>, Option<B>) = (None, None);
    Box::pin(std::future::poll_fn(move |context| {
        if first_output.is_none()
            && let Poll::Ready(value) = first.as_mut().poll(context)
        {
            first_output = Some(value);
        }
        if second_output.is_none()
            && let Poll::Ready(value) = second.as_mut().poll(context)
        {
            second_output = Some(value);
        }
        match (first_output.take(), second_output.take()) {
            (Some(one), Some(two)) => Poll::Ready((one, two)),
            (one, two) => {
                first_output = one;
                second_output = two;
                Poll::Pending
            }
        }
    }))
}

/// A promise awaited by more than one caller: the future runs once, polled by whichever waiter
/// polls first, and every waiter gets its output (a `Promise` kept in a `Map` and awaited again).
pub struct SharedPromise<T>(Arc<Mutex<Slot<T>>>);

/// What a shared promise holds: the future still running, or what it gave.
enum Slot<T> {
    Running(BoxFuture<'static, T>),
    Done(T),
}

impl<T> Clone for SharedPromise<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: Clone + Send + 'static> SharedPromise<T> {
    /// The promise of `future`, which runs when a waiter first polls it.
    #[must_use]
    pub fn new(future: BoxFuture<'static, T>) -> Self {
        Self(Arc::new(Mutex::new(Slot::Running(future))))
    }

    /// `await promise`: its output once the future is done.
    pub fn wait(&self) -> BoxFuture<'static, T> {
        let slot = Arc::clone(&self.0);
        Box::pin(std::future::poll_fn(move |context| {
            let mut slot = slot.lock().unwrap_or_else(PoisonError::into_inner);
            let value = match &mut *slot {
                Slot::Done(value) => return Poll::Ready(value.clone()),
                Slot::Running(future) => match future.as_mut().poll(context) {
                    Poll::Ready(value) => value,
                    Poll::Pending => return Poll::Pending,
                },
            };
            *slot = Slot::Done(value.clone());
            Poll::Ready(value)
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::future::{Future, ready};
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use super::{SharedPromise, promise_all, promise_all_pair};
    use crate::ports::{BoxFuture, now};

    /// A future that waits for one poll before it answers.
    struct Later(Option<u8>, bool);

    impl Future for Later {
        type Output = u8;
        fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<u8> {
            if self.1 {
                Poll::Ready(self.0.take().unwrap_or_default())
            } else {
                self.1 = true;
                context.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    // `await Promise.all([Promise.resolve(1), later(2), Promise.resolve(3)])` is `[1, 2, 3]`.
    #[test]
    fn outputs_keep_the_order_of_the_futures() {
        let futures: Vec<BoxFuture<'_, u8>> = vec![
            Box::pin(ready(1)),
            Box::pin(Later(Some(2), false)),
            Box::pin(ready(3)),
        ];
        let mut all = promise_all(futures);
        let waker = std::task::Waker::noop();
        let mut context = Context::from_waker(waker);
        assert_eq!(all.as_mut().poll(&mut context), Poll::Pending);
        assert_eq!(all.as_mut().poll(&mut context), Poll::Ready(vec![1, 2, 3]));
    }

    #[test]
    fn a_pair_of_different_types_comes_back_together() {
        let mut both = promise_all_pair(Box::pin(Later(Some(7), false)), Box::pin(ready("a")));
        let mut context = Context::from_waker(std::task::Waker::noop());
        assert_eq!(both.as_mut().poll(&mut context), Poll::Pending);
        assert_eq!(both.as_mut().poll(&mut context), Poll::Ready((7, "a")));
    }

    #[test]
    fn nothing_to_wait_for_is_ready_at_once() {
        assert_eq!(now(promise_all::<u8>(Vec::new())), Some(Vec::new()));
    }

    // `const p = later(5); await Promise.all([p, p])` runs `later` once and gives `[5, 5]`.
    #[test]
    fn a_shared_promise_runs_once_for_every_waiter() {
        let shared = SharedPromise::new(Box::pin(Later(Some(5), false)));
        let mut both = promise_all(vec![shared.wait(), shared.wait()]);
        let mut context = Context::from_waker(std::task::Waker::noop());
        assert_eq!(both.as_mut().poll(&mut context), Poll::Pending);
        assert_eq!(both.as_mut().poll(&mut context), Poll::Ready(vec![5, 5]));
        assert_eq!(now(shared.wait()), Some(5));
    }
}
