use std::{fmt::Debug, hash::Hash, ops::Deref};

use futures::Stream;

use crate::{manager::TopicManager, storages::Storage};

pub trait Topic<S, K>
where
    Self: Send + 'static,
    Self::Output: Send + 'static,
    Self::Error: Send + 'static,
    Self::References: Deref<Target = Self::Output> + for<'a> From<&'a Self::Output> + 'static,
    Self::Storage: Storage<K, Self::Output> + 'static,
{
    type Output;

    type Error;

    type References;

    type Storage;

    fn topic_id(&self) -> impl Debug + Hash {}

    fn storage(&self) -> Self::Storage;

    #[allow(unused_variables)]
    fn mount(&mut self, manager: TopicManager<S>, storage: Self::Storage) -> impl Stream<Item = Result<(), Self::Error>> + Send + 'static;
}
