use std::{
    any::{Any, TypeId},
    collections::HashMap,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use futures::stream::BoxStream;

use crate::{SharedStream, VLock};

#[derive(Debug)]
pub struct TopicManager<S>
where
    S: 'static,
{
    store: S,
    topics: HashMap<(TypeId, String), Box<dyn Any + Send + Sync>>,
    drop_lock: VLock,
}

impl<S> TopicManager<S>
where
    S: 'static,
{
    pub fn new(store: S) -> Self {
        Self {
            store,
            topics: HashMap::new(),
            drop_lock: VLock::new(),
        }
    }

    pub fn topic<T>(&mut self, topic: T) -> TopicToken<T, S>
    where
        T: Topic<'static, S> + Send + Sync + 'static,
        T::Output: Send + Sync + Clone + 'static,
        T::Error: Send + Sync + Clone + 'static,
    {
        TopicToken::new(NonNull::from(self), topic)
    }

    pub fn store(&self) -> &S {
        &self.store
    }
}

pub struct TopicToken<T, S>
where
    T: Topic<'static, S> + Send + Sync + 'static,
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
    T: Topic<'static, S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
    S: 'static,
{
    pub fn new(mut manager: NonNull<TopicManager<S>>, topic: T) -> Self {
        unsafe {
            let ptr = manager.as_mut();

            let _lock = ptr.drop_lock.lock();

            let topic_id = (TypeId::of::<T>(), topic.topic());

            if let Some(topic) = ptr.topics.get(&topic_id) {
                if let Some(topic) = topic.downcast_ref::<Self>() {
                    return topic.clone();
                }
            }

            let token = Self {
                topic_id: topic_id.clone(),
                stream: SharedStream::new(topic.init(ptr)),
                manager: manager.clone(),
                strong: Arc::new(AtomicUsize::new(0)),
            };

            ptr.topics.insert(topic_id, Box::new(token.clone()));

            token
        }
    }
}

unsafe impl<T, S> Send for TopicToken<T, S>
where
    T: Topic<'static, S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
    S: 'static,
{
}

unsafe impl<T, S> Sync for TopicToken<T, S>
where
    T: Topic<'static, S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
    S: 'static,
{
}

impl<T, S> Deref for TopicToken<T, S>
where
    T: Topic<'static, S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
    S: 'static,
{
    type Target = SharedStream<BoxStream<'static, Result<T::Output, T::Error>>>;

    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}

impl<T, S> DerefMut for TopicToken<T, S>
where
    T: Topic<'static, S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
    S: 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.stream
    }
}

impl<T, S> Clone for TopicToken<T, S>
where
    T: Topic<'static, S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
    S: 'static,
{
    fn clone(&self) -> Self {
        self.strong.fetch_add(1, Ordering::SeqCst);
        Self {
            topic_id: self.topic_id.clone(),
            stream: self.stream.clone(),
            manager: self.manager.clone(),
            strong: self.strong.clone(),
        }
    }
}

impl<T, S> Drop for TopicToken<T, S>
where
    T: Topic<'static, S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
    S: 'static,
{
    fn drop(&mut self) {
        if self.strong.fetch_sub(1, Ordering::SeqCst) == 1 {
            unsafe {
                let manager = self.manager.as_mut();
                let _lock = manager.drop_lock.lock();
                manager.topics.remove(&self.topic_id);
            }
        }
    }
}

pub trait Topic<'a, S> {
    type Output;

    type Error;

    fn topic(&self) -> String;

    fn init(&self, manager: &mut TopicManager<S>) -> BoxStream<'a, Result<Self::Output, Self::Error>>;
}
