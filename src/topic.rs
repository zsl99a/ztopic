use std::{
    any::{Any, TypeId},
    collections::HashMap,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{stream::BoxStream, Stream, StreamExt};
use parking_lot::Mutex;
use tokio::task::JoinSet;

use crate::{stream::SharedStream, GLOBAL_BATCH_SIZE, GLOBAL_CAPACITY};

#[derive(Debug)]
pub struct TopicManager<S> {
    store: Arc<S>,
    topics: Arc<Mutex<HashMap<(TypeId, String), Box<dyn Any + Send + Sync>>>>,
}

impl<S> Clone for TopicManager<S> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            topics: self.topics.clone(),
        }
    }
}

impl<S> TopicManager<S>
where
    S: Send + Sync + 'static,
{
    pub fn new(store: S) -> Self {
        Self {
            store: Arc::new(store),
            topics: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn topic<T>(&self, topic: T) -> TopicToken<T, S>
    where
        T: Topic<S> + Send + Sync + 'static,
        T::Output: Send + Sync + Clone + 'static,
        T::Error: Send + Sync + Clone + 'static,
    {
        TopicToken::new(topic, self.clone())
    }

    pub fn store(&self) -> &S {
        &self.store
    }
}

pub struct TopicToken<T, S>
where
    T: Topic<S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
    S: Send + Sync + 'static,
{
    topic_id: (TypeId, String),
    stream: SharedStream<BoxStream<'static, Result<T::Output, T::Error>>>,
    manager: TopicManager<S>,
    strong: Arc<()>,
}

impl<T, S> TopicToken<T, S>
where
    T: Topic<S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
    S: Send + Sync + 'static,
{
    pub fn new(topic: T, manager: TopicManager<S>) -> Self {
        let topics = manager.topics.lock();

        let topic_id = (TypeId::of::<T>(), topic.topic());

        let token = if let Some(topic) = topics.get(&topic_id) {
            if let Some(topic) = topic.downcast_ref::<Self>() {
                topic.clone()
            } else {
                panic!("topic type mismatch")
            }
        } else {
            drop(topics);

            let token = Self {
                topic_id: topic_id.clone(),
                stream: SharedStream::new(topic.init(&manager), topic.capacity(), topic.batch_size()),
                manager: manager.clone(),
                strong: Arc::new(()),
            };

            manager.topics.lock().insert(topic_id, Box::new(token.clone()));

            token
        };

        token
    }

    pub fn spawn(mut self) -> JoinSet<()> {
        let mut join_set = JoinSet::new();
        join_set.spawn(async move { while let Some(_s) = self.next().await {} });
        join_set
    }
}

impl<T, S> Clone for TopicToken<T, S>
where
    T: Topic<S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
    S: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
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
    T: Topic<S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
    S: Send + Sync + 'static,
{
    fn drop(&mut self) {
        let mut lock = self.manager.topics.lock();
        if Arc::strong_count(&self.strong) == 2 {
            lock.remove(&self.topic_id);
        }
    }
}

impl<T, S> Deref for TopicToken<T, S>
where
    T: Topic<S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
    S: Send + Sync + 'static,
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
    S: Send + Sync + 'static,
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
    S: Send + Sync + 'static,
{
    type Item = Result<T::Output, T::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream.poll_next_unpin(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

pub trait Topic<S>
where
    S: Send + Sync + 'static,
{
    type Output;

    type Error;

    fn topic(&self) -> String {
        Default::default()
    }

    fn init(&self, manager: &TopicManager<S>) -> BoxStream<'static, Result<Self::Output, Self::Error>>;

    fn capacity(&self) -> usize {
        unsafe { GLOBAL_CAPACITY }
    }

    fn batch_size(&self) -> usize {
        unsafe { GLOBAL_BATCH_SIZE }
    }
}

#[macro_export]
macro_rules! to_topic_dyn {
    ($self:ident) => {
        $self as &dyn Topic<S, Output = Self::Output, Error = Self::Error>
    };
}
