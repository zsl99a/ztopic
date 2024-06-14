use std::{fmt::Debug, hash::Hash, ops::Deref};

use futures::stream::BoxStream;

use crate::{manager::TopicManager, storages::StorageManager, Storage};

#[allow(unused_variables)]
pub trait Topic<S, K>
where
    K: Clone + Default + Eq + Hash,
    Self: Send + 'static,
    Self::Output: Send + 'static,
    Self::Error: Send + 'static,
    Self::References: Deref<Target = Self::Output> + for<'a> From<&'a Self::Output> + 'static,
    Self::Storage: Storage<Self::Output> + 'static,
{
    type Output;

    type Error;

    type References;

    type Storage;

    fn topic_id(&self) -> impl Debug {}

    fn storage(&self) -> Self::Storage;

    fn mount(&self, manager: TopicManager<S>, storage: StorageManager<K, Self::Output, Self::Storage>) -> BoxStream<'static, Result<(), Self::Error>>;
}
