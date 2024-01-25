use std::{
    pin::Pin,
    sync::atomic::{AtomicPtr, Ordering},
    task::{Context, Poll},
};

use futures::Stream;

use crate::buffer::SharedBuffer;

pub struct SharedStream<S>
where
    S: Stream + Unpin,
    S::Item: Clone,
{
    buffer: AtomicPtr<SharedBuffer<S>>,
    cursor: usize,
    stream_id: usize,
}

impl<S> SharedStream<S>
where
    S: Stream + Unpin,
    S::Item: Clone,
{
    pub fn new(stream: S, capacity: usize, batch_size: usize) -> Self {
        Self {
            buffer: AtomicPtr::new(Box::into_raw(Box::new(SharedBuffer::new(stream, capacity, batch_size)))),
            cursor: 0,
            stream_id: 0,
        }
    }

    pub fn insert(&self, item: S::Item) {
        self.buffer_mut().insert(item);
    }
}

impl<S> SharedStream<S>
where
    S: Stream + Unpin,
    S::Item: Clone,
{
    fn buffer(&self) -> &SharedBuffer<S> {
        unsafe { &**self.buffer.as_ptr() }
    }

    fn buffer_mut(&self) -> &mut SharedBuffer<S> {
        unsafe { &mut **self.buffer.as_ptr() }
    }
}

impl<S> Clone for SharedStream<S>
where
    S: Stream + Unpin,
    S::Item: Clone,
{
    fn clone(&self) -> Self {
        Self {
            buffer: AtomicPtr::new(self.buffer.load(Ordering::Relaxed)),
            cursor: self.buffer().new_stream_cursor(),
            stream_id: self.buffer_mut().new_stream_id(),
        }
    }
}

impl<S> Drop for SharedStream<S>
where
    S: Stream + Unpin,
    S::Item: Clone,
{
    fn drop(&mut self) {
        self.buffer_mut().drop_stream(self.stream_id);
    }
}

impl<S> Stream for SharedStream<S>
where
    S: Stream + Unpin,
    S::Item: Clone,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let poll = self.buffer_mut().poll_receive(cx, self.cursor, self.stream_id);

        if poll.is_ready() {
            self.cursor += 1;
            if self.cursor >= self.buffer().capacity() {
                self.cursor = 0;
            }
        }

        poll
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let buffer = self.buffer();

        if buffer.cursor() == self.cursor {
            (0, None)
        } else {
            let len = if buffer.cursor() > self.cursor {
                buffer.cursor() - self.cursor
            } else {
                buffer.capacity() - self.cursor + buffer.cursor()
            };
            (len, None)
        }
    }
}
