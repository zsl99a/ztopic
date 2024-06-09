use std::{any::Any, collections::HashMap, sync::Arc};

use parking_lot::Mutex;

use crate::{token::TopicToken, topic::Topic};

type AnyTopic = Box<dyn Any + Send + Sync>;

#[derive(Debug)]
pub struct TopicManager<S> {
    store: Arc<S>,
    topics: Arc<Mutex<HashMap<String, Option<AnyTopic>>>>,
}

impl<S> Clone for TopicManager<S> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            topics: self.topics.clone(),
        }
    }
}

impl<S> TopicManager<S> {
    pub fn new(store: S) -> Self {
        Self {
            store: Arc::new(store),
            topics: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn store(&self) -> &S {
        &self.store
    }

    pub fn topic<T, K>(&self, topic: T) -> TopicToken<T, S, K>
    where
        T: Topic<S, K>,
        T::Storage: Sync + Unpin,
        S: Send + Sync + 'static,
        K: Default + 'static,
    {
        TopicToken::<T, S, K>::new(topic, self.clone())
    }

    pub(crate) fn topics(&self) -> &Mutex<HashMap<String, Option<AnyTopic>>> {
        &self.topics
    }
}
