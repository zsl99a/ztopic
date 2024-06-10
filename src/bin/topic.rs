use std::time::Duration;

use anyhow::Error;
use futures::{Stream, StreamExt, TryStreamExt};
use jemallocator::Jemalloc;
use ztopic::{
    operators::Interval,
    references::RawRef,
    storages::{Broadcast, Storage},
    Topic, TopicManager,
};

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() {
    let manager = TopicManager::new(37);

    let mut my = manager.topic(MyTopic2::new("demo", 3));

    tokio::spawn(async move {
        let mut idx = 0;

        while let Some(msg) = my.next().await {
            match msg {
                Ok(msg) => {
                    println!("{:?}", msg)
                }
                Err(error) => {
                    eprintln!("{:?}", error)
                }
            }

            idx += 1;
            if idx >= 3 {
                break;
            }
        }
    });

    tokio::time::sleep(Duration::from_secs(5)).await;
}

pub struct MyTopic;

impl Default for MyTopic {
    fn default() -> Self {
        Self
    }
}

impl Topic<usize, ()> for MyTopic {
    type Output = String;

    type Error = Error;

    type References = RawRef<String>;

    type Storage = Broadcast<(), Self::Output>;

    fn storage(&self) -> Self::Storage {
        Broadcast::new(4096)
    }

    fn mount(&mut self, manager: TopicManager<usize>, mut storage: Self::Storage) -> impl Stream<Item = Result<(), Self::Error>> + Send + 'static {
        manager.topic(Interval::new(Duration::from_secs(1))).into_stream().map(move |_| {
            storage.insert(String::from("hello world"));
            Ok(())
        })
    }
}

#[derive(Default)]
pub struct MyTopic2 {
    args: (String, u8),
}

impl MyTopic2 {
    pub fn new(naem: &str, uid: u8) -> Self {
        Self {
            args: (String::from(naem), uid),
        }
    }
}

impl Topic<usize, ()> for MyTopic2 {
    type Output = String;

    type Error = Error;

    type References = RawRef<String>;

    type Storage = Broadcast<(), Self::Output>;

    fn topic_id(&self) -> impl std::fmt::Debug + std::hash::Hash {
        self.args.clone()
    }

    fn storage(&self) -> Self::Storage {
        Broadcast::new(4096)
    }

    fn mount(&mut self, manager: TopicManager<usize>, mut storage: Self::Storage) -> impl Stream<Item = Result<(), Self::Error>> + Send + 'static {
        manager.topic(MyTopic).into_stream().map(move |event| match event {
            Ok(event) => {
                storage.insert(format!("{}, {}", *event, rand::random::<u8>()));
                Ok(())
            }
            Err(error) => Err(error),
        })
    }
}
