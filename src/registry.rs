use std::{
    sync::atomic::{AtomicUsize, Ordering},
    task::Waker,
};

use dashmap::DashMap;

#[derive(Debug)]
pub struct Registry {
    wakers: DashMap<usize, Option<Waker>>,
    next_stream_id: AtomicUsize,
    strong_count: AtomicUsize,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            wakers: DashMap::new(),
            next_stream_id: AtomicUsize::new(1),
            strong_count: AtomicUsize::new(1),
        }
    }
}

impl Registry {
    pub fn new_stream(&self) -> usize {
        self.strong_count.fetch_add(1, Ordering::Release);
        self.next_stream_id.fetch_add(1, Ordering::Release)
    }

    pub fn drop_stream(&self, stream_id: usize) {
        self.strong_count.fetch_sub(1, Ordering::Release);
        self.wakers.remove(&stream_id);
        std::thread::yield_now();
    }

    pub fn strong_count(&self) -> usize {
        self.strong_count.load(Ordering::Acquire)
    }

    pub fn wake_all(&self) {
        self.wakers.alter_all(|_key, waker| {
            if let Some(waker) = waker {
                waker.wake();
            }
            None
        })
    }

    pub fn set_waker(&self, stream_id: usize, waker: Waker) {
        self.wakers.insert(stream_id, Some(waker));
    }
}
