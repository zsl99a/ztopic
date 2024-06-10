use std::{fmt::Debug, hash::Hash, marker::PhantomData};

use futures::{stream::BoxStream, StreamExt};

use crate::{
    manager::TopicManager,
    references::RawRef,
    storages::{Broadcast, StorageManager},
    topic::Topic,
};

pub struct Empty<I, O, E>
where
    I: Debug + Hash,
{
    topic_id: I,
    _marker: PhantomData<(O, E)>,
}

impl<I, O, E> Empty<I, O, E>
where
    I: Debug + Hash,
{
    pub fn new(topic_id: I) -> Self {
        Self {
            topic_id,
            _marker: PhantomData,
        }
    }
}

impl<I, O, E, S> Topic<S, ()> for Empty<I, O, E>
where
    I: Debug + Hash + Clone + Send + 'static,
    O: Send + 'static,
    E: Send + 'static,
    S: Send + 'static,
{
    type Output = O;

    type Error = E;

    type References = RawRef<Self::Output>;

    type Storage = Broadcast<Self::Output>;

    fn topic_id(&self) -> impl Debug + Hash {
        self.topic_id.clone()
    }

    fn storage(&self) -> Self::Storage {
        Broadcast::new(1024)
    }

    fn mount(&self, _: TopicManager<S>, _: StorageManager<(), Self::Output, Self::Storage>) -> BoxStream<'static, Result<(), Self::Error>> {
        futures::stream::empty().boxed()
    }
}
