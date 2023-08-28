use std::{
    convert::Infallible,
    time::{Duration, Instant},
};

use futures::{
    stream::{select_all, BoxStream},
    StreamExt,
};
use helium::{Interval, Topic, TopicManager};

#[tokio::main]
async fn main() {
    let mut manager = TopicManager::new(());

    let mut topic = manager
        .topic(IntervalMulti::new(Duration::from_millis(10)))
        .filter_map(|i| async move {
            match i {
                Ok(i) => {
                    println!("i = {:?}", i);
                    Some(i)
                }
                Err(e) => {
                    println!("e = {:?}", e);
                    None
                }
            }
        })
        .boxed();

    tokio::spawn(async move {
        while let Some(i) = topic.next().await {
            println!("a = {:?}", i);
        }
    });

    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

        let mut topic = manager.topic(IntervalMulti::new(Duration::from_secs(3)));

        while let Some(i) = topic.next().await {
            println!("b = {:?}", i);
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(u64::MAX)).await;
}

pub struct IntervalMulti {
    dur: Duration,
}

impl IntervalMulti {
    pub fn new(dur: Duration) -> Self {
        Self { dur }
    }
}

impl<S: 'static> Topic<S> for IntervalMulti {
    type Output = Instant;

    type Error = Infallible;

    fn topic(&self) -> String {
        format!("{:?}", self.dur)
    }

    fn init(&self, manager: &mut TopicManager<S>) -> BoxStream<'static, Result<Self::Output, Self::Error>> {
        let interval1 = manager.topic(Interval::new(self.dur));
        let interval2 = manager.topic(Interval::new(self.dur));
        select_all(vec![interval1, interval2]).boxed()
    }
}
