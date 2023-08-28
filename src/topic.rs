use std::{
    any::{Any, TypeId},
    collections::HashMap,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::NonNull,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

use futures::{stream::BoxStream, Stream, StreamExt};
use tokio::task::JoinSet;

use crate::{SharedStream, VLock, VLockGuard, GLOBAL_BATCH_SIZE, GLOBAL_CAPACITY};

#[derive(Debug)]
pub struct TopicManager<S> {
    store: S,
    topics: HashMap<(TypeId, String), Box<dyn Any + Send + Sync>>,
    creating_lock: VLock,
    droping_lock: VLock,
}

impl<S> TopicManager<S> {
    pub fn new(store: S) -> Self {
        Self {
            store,
            topics: HashMap::new(),
            creating_lock: VLock::new(),
            droping_lock: VLock::new(),
        }
    }

    pub fn topic<T>(&mut self, topic: T) -> TopicToken<T, S>
    where
        T: Topic<S> + Send + Sync + 'static,
        T::Output: Send + Sync + Clone + 'static,
        T::Error: Send + Sync + Clone + 'static,
    {
        TopicToken::new(topic, NonNull::from(self))
    }

    pub fn store(&self) -> &S {
        &self.store
    }
}

impl<S> TopicManager<S> {
    fn creating_lock(&self) -> (Option<VLockGuard>, Option<VLockGuard>) {
        let creating = self.creating_lock.try_lock();
        let droping = match &creating {
            Some(_) => Some(self.droping_lock.lock()),
            _ => None,
        };
        (creating, droping)
    }

    fn droping_lock(&self) -> VLockGuard {
        self.droping_lock.lock()
    }
}

pub struct TopicToken<T, S>
where
    T: Topic<S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
    S: 'static,
{
    topic_id: (TypeId, String),
    stream: SharedStream<BoxStream<'static, Result<T::Output, T::Error>>>,
    manager: NonNull<TopicManager<S>>,
    strong: Arc<AtomicUsize>,
}

impl<T, S> TopicToken<T, S>
where
    T: Topic<S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
{
    pub fn new(topic: T, mut manager: NonNull<TopicManager<S>>) -> Self {
        unsafe {
            let ptr = manager.as_mut();

            let _lock = ptr.creating_lock();

            let topic_id = (TypeId::of::<T>(), topic.topic());

            if let Some(topic) = ptr.topics.get(&topic_id) {
                if let Some(topic) = topic.downcast_ref::<Self>() {
                    return topic.clone();
                }
            }

            let token = Self {
                topic_id: topic_id.clone(),
                stream: SharedStream::new(topic.init(ptr), topic.capacity(), topic.batch_size()),
                manager,
                strong: Arc::new(AtomicUsize::new(0)),
            };

            ptr.topics.insert(topic_id, Box::new(token.clone()));

            token
        }
    }

    pub fn spawn(mut self) -> JoinSet<()> {
        let mut join_set = JoinSet::new();
        join_set.spawn(async move { while let Some(_s) = self.next().await {} });
        join_set
    }
}

unsafe impl<T, S> Send for TopicToken<T, S>
where
    T: Topic<S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
{
}

unsafe impl<T, S> Sync for TopicToken<T, S>
where
    T: Topic<S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
{
}

impl<T, S> Deref for TopicToken<T, S>
where
    T: Topic<S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
{
    type Target = SharedStream<BoxStream<'static, Result<T::Output, T::Error>>>;

    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}

impl<T, S> DerefMut for TopicToken<T, S>
where
    T: Topic<S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.stream
    }
}

impl<T, S> Stream for TopicToken<T, S>
where
    T: Topic<S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
{
    type Item = Result<T::Output, T::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream.poll_next_unpin(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

impl<T, S> Clone for TopicToken<T, S>
where
    T: Topic<S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
{
    fn clone(&self) -> Self {
        self.strong.fetch_add(1, Ordering::SeqCst);
        Self {
            topic_id: self.topic_id.clone(),
            stream: self.stream.clone(),
            manager: self.manager,
            strong: self.strong.clone(),
        }
    }
}

impl<T, S> Drop for TopicToken<T, S>
where
    T: Topic<S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
{
    fn drop(&mut self) {
        if self.strong.fetch_sub(1, Ordering::SeqCst) == 1 {
            unsafe {
                let manager = self.manager.as_mut();
                let _lock = manager.droping_lock();
                if self.strong.load(Ordering::SeqCst) == 0 {
                    manager.topics.remove(&self.topic_id);
                }
            }
        }
    }
}

pub trait Topic<S> {
    type Output;

    type Error;

    fn topic(&self) -> String {
        Default::default()
    }

    fn init(&self, manager: &mut TopicManager<S>) -> BoxStream<'static, Result<Self::Output, Self::Error>>;

    fn capacity(&self) -> usize {
        unsafe { GLOBAL_CAPACITY }
    }

    fn batch_size(&self) -> usize {
        unsafe { GLOBAL_BATCH_SIZE }
    }
}
