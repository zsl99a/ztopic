use std::{
    collections::HashMap,
    pin::Pin,
    sync::atomic::{AtomicPtr, Ordering},
    task::{Context, Poll, Waker},
};

use futures::{Sink, SinkExt, Stream, StreamExt};

use crate::VLock;

pub(crate) static mut GLOBAL_CAPACITY: usize = 128;
pub(crate) static mut GLOBAL_BATCH_SIZE: usize = 16;

pub fn set_capacity(capacity: usize) {
    unsafe {
        GLOBAL_CAPACITY = capacity;
    }
}

pub fn set_batch_size(batch_size: usize) {
    unsafe {
        GLOBAL_BATCH_SIZE = batch_size;
    }
}

pub struct SharedBuffer<St>
where
    St: Stream + Unpin,
{
    stream: St,
    buffer: Vec<Option<St::Item>>,
    cursor: usize,
    stream_id_last: usize,
    batch_size: usize,
    // Warning:
    // 任何情况下都应该先拿 stream_lock 再拿 wakers_lock, 否则可能会死锁
    stream_lock: VLock,
    wakers: HashMap<usize, Waker>,
    wakers_lock: VLock,

    #[cfg(feature = "elapsed")]
    elapsed: std::time::Duration,
}

impl<St> SharedBuffer<St>
where
    St: Stream + Unpin,
    St::Item: Clone,
{
    pub fn new(stream: St, capacity: usize, batch_size: usize) -> Self {
        assert!(capacity > 1);
        assert!(batch_size > 0);
        assert!(if capacity >= 3 { capacity / batch_size >= 3 } else { true });

        Self {
            stream,
            buffer: vec![None; capacity],
            cursor: 0,
            stream_id_last: 0,
            batch_size,
            stream_lock: VLock::new(),
            wakers: HashMap::new(),
            wakers_lock: VLock::new(),

            #[cfg(feature = "elapsed")]
            elapsed: std::time::Duration::from_secs(0),
        }
    }

    #[inline]
    fn poll_receive(&mut self, cx: &mut Context<'_>, stream_cursor: usize, stream_id: usize) -> Poll<Option<St::Item>> {
        if stream_cursor == self.cursor {
            if let Some(_lock) = self.stream_lock.try_lock() {
                #[cfg(feature = "elapsed")]
                let ins = std::time::Instant::now();

                let mut idx = 0;

                while let Poll::Ready(Some(item)) = self.stream.poll_next_unpin(cx) {
                    self.buffer[self.cursor] = Some(item);

                    self.cursor();

                    idx += 1;
                    if idx >= self.batch_size {
                        break;
                    }
                }

                #[cfg(feature = "elapsed")]
                {
                    self.elapsed = ins.elapsed();
                }

                if stream_cursor != self.cursor {
                    self.wake_all();
                    return Poll::Ready(self.buffer[stream_cursor].clone());
                }
            }

            self.push_waker(cx, stream_id);
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
    fn push_waker(&mut self, cx: &mut Context<'_>, stream_id: usize) {
        let _lock = self.wakers_lock.lock();
        self.wakers.insert(stream_id, cx.waker().clone());
    }

    #[inline]
    fn wake_all(&mut self) {
        let _lock = self.wakers_lock.lock();
        for (_, waker) in self.wakers.drain() {
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
    stream_id: usize,
}

impl<St> Clone for SharedStream<St>
where
    St: Stream + Unpin,
    St::Item: Clone,
{
    fn clone(&self) -> Self {
        let shared = self.shared();
        Self {
            buffer: AtomicPtr::new(self.buffer.load(Ordering::Relaxed)),
            cursor: {
                let cursor = if shared.cursor == 0 { shared.buffer.len() - 1 } else { shared.cursor - 1 };
                if shared.buffer[cursor].is_some() {
                    cursor
                } else {
                    shared.cursor
                }
            },
            stream_id: {
                shared.stream_id_last += 1;
                shared.stream_id_last
            },
        }
    }
}

impl<St> SharedStream<St>
where
    St: Stream + Unpin,
    St::Item: Clone,
{
    pub fn new(stream: St, capacity: usize, batch_size: usize) -> Self {
        Self {
            buffer: AtomicPtr::new(Box::into_raw(Box::new(SharedBuffer::new(stream, capacity, batch_size)))),
            cursor: 0,
            stream_id: 0,
        }
    }

    #[inline]
    pub fn insert(&mut self, item: St::Item) {
        self.shared().insert(item);
    }

    #[inline]
    #[cfg(feature = "elapsed")]
    pub fn elapsed(&self) -> std::time::Duration {
        self.shared().elapsed
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    fn shared(&self) -> &mut SharedBuffer<St> {
        unsafe { &mut **self.buffer.as_ptr() }
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
            let buffer = &mut **self.buffer.as_ptr();

            let poll = buffer.poll_receive(cx, self.cursor, self.stream_id);

            if poll.is_ready() {
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        let buffer = unsafe { &**self.buffer.as_ptr() };
        if buffer.cursor == self.cursor {
            (0, None)
        } else {
            let len = if buffer.cursor > self.cursor {
                buffer.cursor - self.cursor
            } else {
                buffer.buffer.len() - self.cursor + buffer.cursor
            };
            (len, None)
        }
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
