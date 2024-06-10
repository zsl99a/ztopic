use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{Stream, StreamExt};

use crate::{manager::TopicManager, storages::Storage, stream::MultipleStream, topic::Topic};

pub struct TopicToken<T, S, K>
where
    T: Topic<S, K>,
    T::Storage: Unpin,
    S: Send + Sync + 'static,
    K: Default + 'static,
{
    topic_id: String,
    manager: TopicManager<S>,
    stream: MultipleStream<T, S, K>,
}

impl<T, S, K> Clone for TopicToken<T, S, K>
where
    T: Topic<S, K>,
    T::Storage: Unpin,
    S: Send + Sync + 'static,
    K: Default + 'static,
{
    fn clone(&self) -> Self {
        Self {
            topic_id: self.topic_id.clone(),
            manager: self.manager.clone(),
            stream: self.stream.clone(),
        }
    }
}

impl<T, S, K> Drop for TopicToken<T, S, K>
where
    T: Topic<S, K>,
    T::Storage: Unpin,
    S: Send + Sync + 'static,
    K: Default + 'static,
{
    fn drop(&mut self) {
        let topic_id = self.topic_id.clone();
        let manager = self.manager.clone();
        let inner = self.stream.inner().clone();
        tokio::spawn(async move {
            let mut lock = manager.topics().lock();
            if Arc::strong_count(&inner) == 2 {
                lock.remove(&topic_id);
            }
        });
    }
}

impl<T, S, K> TopicToken<T, S, K>
where
    T: Topic<S, K>,
    T::Storage: Sync + Unpin,
    S: Send + Sync + 'static,
    K: Default + 'static,
{
    pub(crate) fn new(topic: T, manager: TopicManager<S>) -> Self {
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

                let token = Self {
                    topic_id: topic_id.clone(),
                    manager: manager.clone(),
                    stream: MultipleStream::new(topic, manager.clone()),
                };

                manager.topics().lock().insert(topic_id.clone(), Some(Box::new(token.clone())));

                return token;
            }
        }
    }

    pub fn with_key(mut self, key: K) -> Self {
        self.stream.storage().with_key(key);
        self
    }
}

impl<T, S, K> Stream for TopicToken<T, S, K>
where
    T: Topic<S, K>,
    T::Storage: Unpin,
    S: Send + Sync + 'static,
    K: Default + 'static,
{
    type Item = Result<T::References, T::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream.poll_next_unpin(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}
