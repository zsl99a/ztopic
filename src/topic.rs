use std::{
    any::Any,
    collections::{HashMap, HashSet},
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
pub struct TopicManager<S>
where
    S: Send + Sync + 'static,
{
    store: Arc<S>,
    topics: Arc<Mutex<HashMap<String, Box<dyn Any + Send + Sync>>>>,
    adding_topics: Arc<Mutex<HashSet<String>>>,
}

impl<S> Clone for TopicManager<S>
where
    S: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            topics: self.topics.clone(),
            adding_topics: self.adding_topics.clone(),
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
            adding_topics: Arc::new(Mutex::new(HashSet::new())),
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

    pub(crate) fn topics(&self) -> Vec<String> {
        self.topics.lock().iter().map(|(k, _)| k.clone()).collect()
    }
}

pub struct TopicToken<T, S>
where
    T: Topic<S> + Send + Sync + 'static,
    T::Output: Send + Sync + Clone + 'static,
    T::Error: Send + Sync + Clone + 'static,
    S: Send + Sync + 'static,
{
    topic_id: String,
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
        let topic_id = format!("{} {{ {} }}", std::any::type_name::<T>(), topic.topic());

        loop {
            let topics = manager.topics.lock();
            let mut adding_topics = manager.adding_topics.lock();

            if let Some(topic) = topics.get(&topic_id) {
                // 如果 topic 已经存在了，就不需要再次创建了
                return topic.downcast_ref::<Self>().unwrap().clone();
            } else if adding_topics.contains(&topic_id) {
                // 如果 topic 还未存在，但正在创建中，就等待

                drop(topics);
                drop(adding_topics);
                std::thread::yield_now();
            } else {
                // 如果 topic 还未存在，就创建一个新的

                adding_topics.insert(topic_id.clone());

                // topic.init 函数内部会获取 manager.topics.lock()，所以这里需要drop掉topic
                // 注意，这里需要先 insert adding_topics，再 drop topics，否则会导致重复创建相同的Topic
                drop(topics);
                drop(adding_topics);

                let token = Self {
                    topic_id: topic_id.clone(),
                    stream: SharedStream::new(topic.init(&manager), topic.capacity(), topic.batch_size()),
                    manager: manager.clone(),
                    strong: Arc::new(()),
                };

                manager.topics.lock().insert(topic_id.clone(), Box::new(token.clone()));
                manager.adding_topics.lock().remove(&topic_id);

                return token;
            }
        }
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
        if Arc::strong_count(&self.strong) == 2 {
            let strong = self.strong.clone();
            let topic_id = self.topic_id.clone();
            let topics = self.manager.topics.clone();
            tokio::spawn(async move {
                let mut lock = topics.lock();
                if Arc::strong_count(&strong) == 2 {
                    lock.remove(&topic_id);
                }
            });
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
