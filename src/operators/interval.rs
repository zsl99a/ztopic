use std::{
    convert::Infallible,
    fmt::Debug,
    hash::Hash,
    time::{Duration, Instant},
};

use futures::Stream;

use crate::{
    manager::TopicManager,
    references::RawRef,
    storages::{broadcast::Broadcast, Storage},
    topic::Topic,
};

pub struct Interval {
    duration: Duration,
}

impl Interval {
    pub fn new(duration: Duration) -> Self {
        Self { duration }
    }
}

impl<S> Topic<S, ()> for Interval {
    type Output = Instant;

    type Error = Infallible;

    type References = RawRef<Self::Output>;

    type Storage = Broadcast<(), Self::Output>;

    fn topic_id(&self) -> impl Debug + Hash {
        self.duration
    }

    fn storage(&self) -> Self::Storage {
        Broadcast::new(128)
    }

    fn mount(&mut self, _manager: TopicManager<S>, mut storage: Self::Storage) -> impl Stream<Item = Result<(), Self::Error>> + Send + 'static {
        let duration = self.duration;
        async_stream::stream! {
            let mut ins = Instant::now();
            loop {
                storage.insert_with_default(ins);
                yield Ok(());
                ins = Instant::now();
                tokio::time::sleep(duration).await;
            }
        }
    }
}
