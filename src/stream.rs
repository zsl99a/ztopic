use std::{
    collections::HashMap,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll, Waker},
};

use futures::{stream::BoxStream, Stream, StreamExt};
use parking_lot::Mutex;
use pin_project::pin_project;

use crate::{
    manager::TopicManager,
    storages::{Storage, SyncCell},
    topic::Topic,
};

#[pin_project]
pub struct MultipleStream<T, S, K>
where
    T: Topic<S, K>,
    S: Send + 'static,
    K: Default + 'static,
{
    inner: Arc<SyncCell<Inner<T, S, K>>>,
    storage: T::Storage,
    cursor: usize,
    stream_id: usize,
}

impl<T, S, K> Clone for MultipleStream<T, S, K>
where
    T: Topic<S, K>,
    S: Send + 'static,
    K: Default + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            storage: self.storage.clone(),
            cursor: self.storage.get_prev_cursor(),
            stream_id: self.new_stream_id(),
        }
    }
}

impl<T, S, K> MultipleStream<T, S, K>
where
    T: Topic<S, K>,
    S: Send + 'static,
    K: Default + 'static,
{
    pub fn new(mut topic: T, manager: TopicManager<S>) -> Self {
        let storage = topic.storage();
        let stream = topic.mount(manager, storage.clone()).boxed();
        Self {
            inner: Arc::new(SyncCell::new(Inner::new(stream))),
            storage,
            cursor: 0,
            stream_id: 0,
        }
    }

    pub fn with_key(&mut self, key: K) {
        self.storage.with_key(key);
    }

    pub(crate) fn inner(&self) -> &Arc<SyncCell<Inner<T, S, K>>> {
        &self.inner
    }

    fn stream(&self) -> &mut BoxStream<'static, Result<(), T::Error>> {
        &mut self.inner.get_mut().stream
    }

    fn new_stream_id(&self) -> usize {
        self.inner.get().next_stream_id.fetch_add(1, Ordering::Release)
    }

    fn wake_all(&self) {
        for (_, waker) in self.inner.get().wakers.lock().drain() {
            waker.wake();
        }
    }
}

impl<T, S, K> Stream for MultipleStream<T, S, K>
where
    T: Topic<S, K>,
    S: Send + 'static,
    K: Default + 'static,
{
    type Item = Result<T::References, T::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if let Some((item, cursor)) = self.storage.get_item(self.cursor).map(|(item, cursor)| (T::References::from(item), cursor)) {
                self.cursor = cursor;
                return Poll::Ready(Some(Ok(item)));
            }

            if let Some(_lock) = self.inner.get().streaming.try_lock() {
                let mut index = 0;
                loop {
                    match self.stream().poll_next_unpin(cx) {
                        Poll::Ready(Some(Ok(_))) => index += 1,
                        Poll::Ready(Some(Err(error))) => return Poll::Ready(Some(Err(error))),
                        Poll::Ready(None) => return Poll::Ready(None),
                        Poll::Pending => break,
                    }
                }
                if index > 0 {
                    self.wake_all();
                    continue;
                }
            } else {
                self.inner.get().wakers.lock().insert(self.stream_id, cx.waker().clone());
            }

            return Poll::Pending;
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.storage.size_hint(self.cursor), None)
    }
}

pub(crate) struct Inner<T, S, K>
where
    T: Topic<S, K>,
    S: Send + 'static,
    K: Default + 'static,
{
    stream: BoxStream<'static, Result<(), T::Error>>,
    next_stream_id: AtomicUsize,
    streaming: Mutex<()>,
    wakers: Mutex<HashMap<usize, Waker>>,
}

impl<T, S, K> Inner<T, S, K>
where
    T: Topic<S, K>,
    S: Send + 'static,
    K: Default + 'static,
{
    fn new(stream: BoxStream<'static, Result<(), T::Error>>) -> Self {
        Self {
            stream,
            next_stream_id: AtomicUsize::new(1),
            streaming: Mutex::new(()),
            wakers: Mutex::new(HashMap::new()),
        }
    }
}
