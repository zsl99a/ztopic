use std::{fmt::Debug, hash::Hash, marker::PhantomData};

use futures::Stream;

use crate::{global::GLOBAL_CAPACITY, manager::TopicManager, references::RawRef, storages::broadcast::Broadcast, topic::Topic};

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

    type Storage = Broadcast<(), Self::Output>;

    fn topic_id(&self) -> impl Debug + Hash {
        self.topic_id.clone()
    }

    fn storage(&self) -> Self::Storage {
        Broadcast::new(unsafe { GLOBAL_CAPACITY })
    }

    fn mount(&mut self, _manager: TopicManager<S>, _storage: Self::Storage) -> impl Stream<Item = Result<(), Self::Error>> + Send + 'static {
        futures::stream::empty()
    }
}
