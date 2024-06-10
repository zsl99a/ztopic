use std::{
    cmp::Eq,
    hash::Hash,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

use futures::{stream::BoxStream, Stream, StreamExt};
use parking_lot::Mutex;

use crate::{
    common::SyncCell,
    manager::TopicManager,
    storages::{Storage, StorageManager},
    topic::Topic,
};

pub struct TopicToken<T, S, K>
where
    T: Topic<S, K>,
    T::Storage: Storage<T::Output>,
    S: Send + Sync + 'static,
    K: Default + Clone + Hash + Eq + Send + Sync + Unpin + 'static,
{
    inner: Arc<SyncCell<Inner<T, S, K>>>,
    storage: StorageManager<K, T::Output, T::Storage>,
    cursor: usize,
    stream_id: usize,
}

pub(crate) struct Inner<T, S, K>
where
    T: Topic<S, K>,
    T::Storage: Storage<T::Output>,
    S: Send + Sync + 'static,
    K: Default + Clone + Hash + Eq + Send + Sync + Unpin + 'static,
{
    stream: BoxStream<'static, Result<(), T::Error>>,
    topic_id: String,
    manager: TopicManager<S>,
    next_stream_id: AtomicUsize,
    streaming: Mutex<()>,
}

impl<T, S, K> Clone for TopicToken<T, S, K>
where
    T: Topic<S, K>,
    T::Storage: Storage<T::Output>,
    S: Send + Sync + 'static,
    K: Default + Clone + Hash + Eq + Send + Sync + Unpin + 'static,
{
    fn clone(&self) -> Self {
        let this = Self {
            inner: self.inner.clone(),
            storage: self.storage.clone(),
            cursor: self.cursor,
            stream_id: self.new_stream_id(),
        };
        this.storage.new_stream(this.stream_id);
        this
    }
}

impl<T, S, K> Drop for TopicToken<T, S, K>
where
    T: Topic<S, K>,
    T::Storage: Storage<T::Output>,
    S: Send + Sync + 'static,
    K: Default + Clone + Hash + Eq + Send + Sync + Unpin + 'static,
{
    fn drop(&mut self) {
        let stream_id = self.stream_id;
        let storage = self.storage.clone();
        let topic_id = self.inner.topic_id.clone();
        let manager = self.inner.manager.clone();
        let inner = self.inner.clone();
        tokio::spawn(async move {
            let mut lock = manager.topics().lock();
            storage.drop_stream(stream_id);
            if Arc::strong_count(&inner) == 2 {
                println!("drop topic {}", topic_id);
                lock.remove(&topic_id);
            }
        });
    }
}

impl<T, S, K> TopicToken<T, S, K>
where
    T: Topic<S, K>,
    T::Storage: Storage<T::Output>,
    S: Send + Sync + 'static,
    K: Default + Clone + Hash + Eq + Send + Sync + Unpin + 'static,
{
    pub(crate) fn new(mut topic: T, manager: TopicManager<S>) -> Self {
        let topic_id = format!("{}·{:?}", std::any::type_name::<T>(), topic.topic_id());

        loop {
            let mut lock = manager.topics().lock();

            if let Some(topic) = lock.get(&topic_id) {
                if let Some(topic) = topic {
                    return topic.downcast_ref::<Self>().unwrap().clone();
                } else {
                    drop(lock);
                    std::thread::yield_now();
                }
            } else {
                lock.insert(topic_id.clone(), None);
                drop(lock);

                let storage = StorageManager::new(topic.storage());
                let stream = topic.mount(manager.clone(), storage.clone());

                let token = Self {
                    inner: Arc::new(SyncCell::new(Inner {
                        stream,
                        topic_id: topic_id.clone(),
                        manager: manager.clone(),
                        next_stream_id: AtomicUsize::new(1),
                        streaming: Mutex::new(()),
                    })),
                    storage,
                    cursor: 0,
                    stream_id: 0,
                };

                token.storage.new_stream(token.stream_id);

                manager.topics().lock().insert(topic_id.clone(), Some(Box::new(token.clone())));

                return token;
            }
        }
    }

    fn new_stream_id(&self) -> usize {
        self.inner.next_stream_id.fetch_add(1, Ordering::Release)
    }

    pub fn with_key(mut self, stream_key: K) -> Self {
        let lock = self.inner.manager.topics().lock();
        self.storage.with_key(stream_key, self.stream_id);
        drop(lock);
        self
    }
}

impl<T, S, K> Stream for TopicToken<T, S, K>
where
    T: Topic<S, K>,
    T::Storage: Storage<T::Output>,
    S: Send + Sync + 'static,
    K: Default + Clone + Hash + Eq + Send + Sync + Unpin + 'static,
{
    type Item = Result<T::References, T::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if let Some((item, cursor)) = self.storage.get_item(self.cursor).map(|(item, cursor)| (T::References::from(item), cursor)) {
                self.cursor = cursor;
                return Poll::Ready(Some(Ok(item)));
            }

            if let Some(_lock) = self.inner.streaming.try_lock() {
                loop {
                    match self.inner.get_mut().stream.poll_next_unpin(cx) {
                        Poll::Ready(Some(Ok(_))) => {}
                        Poll::Ready(Some(Err(error))) => return Poll::Ready(Some(Err(error))),
                        Poll::Ready(None) => return Poll::Ready(None),
                        Poll::Pending => break,
                    }
                }

                if self.storage.registry().changed() {
                    self.storage.registry_mut().wake_all();
                    continue;
                }
            } else {
                self.storage.register(self.stream_id, cx.waker())
            }

            return Poll::Pending;
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.storage.size_hint(self.cursor), None)
    }
}
