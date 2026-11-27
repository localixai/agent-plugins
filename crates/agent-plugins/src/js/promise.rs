//! JavaScript's `Promise.all`: the futures run together, so a port that is the host's is asked
//! for all of their needs in one step (docs/RUST.md, "Effects").

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

#[cfg(test)]
mod tests {
    use std::future::{Future, ready};
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use super::{promise_all, promise_all_pair};
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
}
