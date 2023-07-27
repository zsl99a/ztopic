use std::{
    pin::Pin,
    sync::atomic::{AtomicPtr, Ordering},
    task::{Context, Poll, Waker},
};

use futures::{Sink, SinkExt, Stream, StreamExt};

use crate::VLock;

pub struct SharedBuffer<St>
where
    St: Stream + Unpin,
{
    stream: St,
    buffer: Vec<Option<St::Item>>,
    cursor: usize,
    // Warning:
    // 任何情况下都应该先拿 stream_lock 再拿 wakers_lock, 否则可能会死锁
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

    #[inline]
    fn poll_receive(&mut self, cx: &mut Context<'_>, stream_cursor: usize) -> Poll<Option<St::Item>> {
        if stream_cursor == self.cursor {
            if let Some(_lock) = self.stream_lock.try_lock() {
                let mut idx = 0;

                while let Poll::Ready(Some(item)) = self.stream.poll_next_unpin(cx) {
                    self.buffer[self.cursor] = Some(item);

                    self.cursor();

                    idx += 1;
                    if idx >= 16 {
                        break;
                    }
                }

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
    fn insert(&mut self, item: St::Item) {
        let _lock = self.stream_lock.lock();
        self.buffer[self.cursor] = Some(item);
        self.cursor();
        self.wake_all();
    }

    #[inline]
    fn cursor(&mut self) {
        self.cursor += 1;
        if self.cursor >= self.buffer.len() {
            self.cursor = 0;
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
    buffer: AtomicPtr<SharedBuffer<St>>,
    cursor: usize,
}

impl<St> Clone for SharedStream<St>
where
    St: Stream + Unpin,
    St::Item: Clone,
{
    fn clone(&self) -> Self {
        Self {
            buffer: AtomicPtr::new(self.buffer.load(Ordering::Relaxed)),
            cursor: self.shared().cursor,
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
            buffer: AtomicPtr::new(Box::into_raw(Box::new(SharedBuffer::new(stream)))),
            cursor: 0,
        }
    }

    #[inline]
    pub fn insert(&mut self, item: St::Item) {
        self.shared().insert(item);
    }

    #[inline]
    fn shared(&self) -> &mut SharedBuffer<St> {
        unsafe { &mut *self.buffer.load(Ordering::Relaxed) }
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

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.poll_receive(cx)
    }
}

impl<St, SinkItem> Sink<SinkItem> for SharedStream<St>
where
    St: Stream + Sink<SinkItem> + Unpin,
    St::Item: Clone,
{
    type Error = St::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.shared().stream.poll_ready_unpin(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: SinkItem) -> Result<(), Self::Error> {
        self.shared().stream.start_send_unpin(item)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.shared().stream.poll_flush_unpin(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.shared().stream.poll_close_unpin(cx)
    }
}
