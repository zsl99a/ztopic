use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};
use parking_lot::Mutex;

pub struct SharedBuffer<S>
where
    S: Stream + Unpin,
{
    stream: S,
    capacity: usize,
    batch_size: usize,

    strong: AtomicUsize,
    next_stream_id: AtomicUsize,

    buffer: Vec<Option<S::Item>>,
    cursor: Mutex<usize>,

    wakers: Mutex<HashMap<usize, Waker>>,
}

impl<S> SharedBuffer<S>
where
    S: Stream + Unpin,
    S::Item: Clone,
{
    pub fn new(stream: S, capacity: usize, batch_size: usize) -> Self {
        assert!(capacity > 1);
        assert!(batch_size > 0);
        assert!(if capacity >= 3 { capacity / batch_size >= 3 } else { true });

        Self {
            stream,
            capacity,
            batch_size,

            strong: AtomicUsize::new(1),
            next_stream_id: AtomicUsize::new(1),

            buffer: vec![None; capacity],
            cursor: Mutex::new(0),

            wakers: Mutex::new(HashMap::new()),
        }
    }
}

macro_rules! update_item {
    ($self:ident, $cursor:ident, $item:ident) => {
        $self.buffer[*$cursor] = Some($item);

        if *$cursor >= $self.capacity - 1 {
            *$cursor = 0;
        } else {
            *$cursor += 1;
        }
    };
}

impl<S> SharedBuffer<S>
where
    S: Stream + Unpin,
    S::Item: Clone,
{
    pub fn poll_receive(&mut self, cx: &mut Context<'_>, stream_cursor: usize, stream_id: usize) -> Poll<Option<S::Item>> {
        if stream_cursor == self.cursor() {
            if let Some(mut cursor) = self.cursor.try_lock() {
                let mut idx = 0;

                while let Poll::Ready(Some(item)) = self.stream.poll_next_unpin(cx) {
                    update_item!(self, cursor, item);

                    idx += 1;
                    if idx >= self.batch_size {
                        break;
                    }
                }

                if stream_cursor != *cursor {
                    self.wake_all();
                    return Poll::Ready(self.buffer[stream_cursor].clone());
                }
            }

            self.insert_waker(stream_id, cx.waker().clone());

            Poll::Pending
        } else {
            Poll::Ready(self.buffer[stream_cursor].clone())
        }
    }

    #[inline]
    pub fn cursor(&self) -> usize {
        unsafe { *self.cursor.data_ptr() }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn insert(&mut self, item: S::Item) {
        let mut cursor = self.cursor.lock();
        update_item!(self, cursor, item);
        self.wake_all()
    }

    #[inline]
    fn insert_waker(&self, stream_id: usize, waker: Waker) {
        self.wakers.lock().insert(stream_id, waker);
    }

    #[inline]
    fn wake_all(&self) {
        let mut lock = self.wakers.lock();
        for (_, waker) in lock.drain() {
            waker.wake();
        }
    }
}

impl<S> SharedBuffer<S>
where
    S: Stream + Unpin,
    S::Item: Clone,
{
    #[inline]
    pub fn new_stream_cursor(&self) -> usize {
        let cursor = self.cursor();
        let cursor = if cursor == 0 { self.capacity - 1 } else { cursor - 1 };
        if self.buffer[cursor].is_none() {
            self.cursor()
        } else {
            cursor
        }
    }

    #[inline]
    pub fn new_stream_id(&self) -> usize {
        self.strong.fetch_add(1, Ordering::AcqRel);
        self.next_stream_id.fetch_add(1, Ordering::Relaxed)
    }

    #[inline]
    pub fn drop_stream(&mut self, stream_id: usize) {
        if self.strong.fetch_sub(1, Ordering::AcqRel) == 1 {
            std::thread::yield_now();
            if self.strong.load(Ordering::Acquire) == 0 {
                unsafe { std::ptr::drop_in_place(self) };
                return;
            }
        }
        self.wakers.lock().remove(&stream_id);
        self.wake_all();
    }
}
