use std::{
    convert::Infallible,
    fmt::Debug,
    time::{Duration, Instant},
};

use futures::stream::{BoxStream, StreamExt};

use crate::{
    manager::TopicManager,
    references::RawRef,
    storages::{Broadcast, StorageManager},
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

    type Storage = Broadcast<Self::Output>;

    fn topic_id(&self) -> impl Debug {
        self.duration
    }

    fn storage(&self) -> Self::Storage {
        Broadcast::new(128)
    }

    fn mount(&self, _: TopicManager<S>, storage: StorageManager<(), Self::Output, Self::Storage>) -> BoxStream<'static, Result<(), Self::Error>> {
        let duration = self.duration;
        async_stream::stream! {
            let mut ins = Instant::now();
            loop {
                storage.scope().insert(ins);
                yield Ok(());
                ins = Instant::now();
                tokio::time::sleep(duration).await;
            }
        }
        .boxed()
    }
}
