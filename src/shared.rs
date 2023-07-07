use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Future, Stream, StreamExt};

pub struct SharedStream<'a, St: ?Sized> {
    stream: &'a mut St,
}

impl<St: ?Sized + Unpin> Unpin for SharedStream<'_, St> {}

impl<St: ?Sized + Stream + Unpin> Future for SharedStream<'_, St> {
    type Output = Option<St::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match Pin::new(&mut self.stream).poll_next_unpin(cx) {
                Poll::Ready(Some(item)) => {
                    return Poll::Ready(Some(item));
                }
                Poll::Ready(None) => {
                    return Poll::Ready(None);
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}
