use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use futures::{channel::mpsc, stream::BoxStream, SinkExt, StreamExt};

use crate::SharedStream;

pub struct TopicManager<S> {
    store: S,
    topics: HashMap<(TypeId, String), Box<dyn Any + Send + Sync>>,
}

impl<S> TopicManager<S> {
    pub fn new(store: S) -> Self {
        Self { store, topics: HashMap::new() }
    }

    pub fn topic<T>(&mut self, topic: T) -> SharedStream<BoxStream<'static, Result<T::Output, T::Error>>>
    where
        T: Topic<'static, S> + Send + Sync + 'static,
        T::Output: Send + Sync + Clone + 'static,
        T::Error: Send + Sync + Clone + 'static,
    {
        let topic_id = (TypeId::of::<T>(), topic.topic());

        if let Some(topic) = self.topics.get(&topic_id) {
            if let Some(topic) = topic.downcast_ref::<SharedStream<BoxStream<'static, Result<T::Output, T::Error>>>>() {
                return topic.clone();
            }
        }

        let stream = SharedStream::new(topic.init(self));

        self.topics.insert(topic_id, Box::new(stream.clone()));

        stream
    }

    pub fn store(&self) -> &S {
        &self.store
    }
}

pub trait Topic<'a, S> {
    type Output;

    type Error;

    fn topic(&self) -> String;

    fn init(&self, manager: &mut TopicManager<S>) -> BoxStream<'a, Result<Self::Output, Self::Error>>;
}

pub struct MyTopic {
    name: String,
}

impl MyTopic {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl<'a, S> Topic<'a, S> for MyTopic {
    type Output = usize;

    type Error = ();

    fn topic(&self) -> String {
        format!("{}", self.name)
    }

    fn init(&self, _manager: &mut TopicManager<S>) -> BoxStream<'a, Result<Self::Output, Self::Error>> {
        let (mut tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            for i in 0..10000 {
                tx.send(i).await.unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
        });

        rx.scan(0, |f, _i| {
            *f += 1;
            futures::future::ready(Some(*f))
        })
        .map(|i| Ok(i))
        .boxed()
    }
}
