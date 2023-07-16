use std::{
    sync::{
        atomic::{AtomicPtr, Ordering},
        Arc,
    },
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};

use crate::VLock;

pub struct SharedBuffer<St>
where
    St: Stream + Unpin,
{
    stream: St,
    buffer: Vec<Option<St::Item>>,
    cursor: usize,
    stream_lock: VLock,
    wakers: Vec<Waker>,
    wakers_lock: VLock,
}

impl<St> SharedBuffer<St>
where
    St: Stream + Unpin,
    St::Item: Clone,
{
    pub fn new(stream: St) -> Self {
        Self {
            stream,
            buffer: vec![None; 128],
            cursor: 0,
            stream_lock: VLock::new(),
            wakers: Vec::new(),
            wakers_lock: VLock::new(),
        }
    }

    fn poll_receive(&mut self, cx: &mut Context<'_>, stream_cursor: usize) -> Poll<Option<St::Item>> {
        if stream_cursor == self.cursor {
            if let Some(_lock) = self.stream_lock.try_lock() {
                let mut idx = 0;

                let mut cursor = self.cursor;

                while let Poll::Ready(Some(item)) = self.stream.poll_next_unpin(cx) {
                    self.buffer[cursor] = Some(item);

                    cursor += 1;
                    if cursor >= self.buffer.len() {
                        cursor = 0;
                    }

                    idx += 1;
                    if idx >= 16 {
                        break;
                    }
                }

                self.cursor = cursor;

                if stream_cursor != self.cursor {
                    self.wake_all();
                    return Poll::Ready(self.buffer[stream_cursor].clone());
                }
            }

            self.push_waker(cx);
            Poll::Pending
        } else {
            Poll::Ready(self.buffer[stream_cursor].clone())
        }
    }

    #[inline]
    fn push_waker(&mut self, cx: &mut Context<'_>) {
        let _lock = self.wakers_lock.lock();
        self.wakers.push(cx.waker().clone());
    }

    #[inline]
    fn wake_all(&mut self) {
        let _lock = self.wakers_lock.lock();
        for waker in self.wakers.drain(..) {
            waker.wake();
        }
    }
}

pub struct SharedStream<St>
where
    St: Stream + Unpin,
    St::Item: Clone,
{
    buffer: Arc<AtomicPtr<SharedBuffer<St>>>,
    cursor: usize,
}

impl<St> Clone for SharedStream<St>
where
    St: Stream + Unpin,
    St::Item: Clone,
{
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
            cursor: unsafe { &mut *self.buffer.load(Ordering::Relaxed) }.cursor,
        }
    }
}

impl<St> SharedStream<St>
where
    St: Stream + Unpin,
    St::Item: Clone,
{
    pub fn new(stream: St) -> Self {
        Self {
            buffer: Arc::new(AtomicPtr::new(Box::into_raw(Box::new(SharedBuffer::new(stream))))),
            cursor: 0,
        }
    }
}

impl<St> SharedStream<St>
where
    St: Stream + Unpin,
    St::Item: Clone,
{
    #[inline]
    fn poll_receive(&mut self, cx: &mut Context<'_>) -> Poll<Option<St::Item>> {
        unsafe {
            let buffer = &mut *self.buffer.load(Ordering::Relaxed);

            let poll = buffer.poll_receive(cx, self.cursor);

            if let Poll::Ready(_) = &poll {
                self.cursor += 1;
                if self.cursor >= buffer.buffer.len() {
                    self.cursor = 0;
                }
            }

            poll
        }
    }
}

impl<St> Stream for SharedStream<St>
where
    St: Stream + Unpin,
    St::Item: Clone,
{
    type Item = St::Item;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.poll_receive(cx)
    }
}
