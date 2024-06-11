use std::time::Duration;

use anyhow::Error;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use jemallocator::Jemalloc;
use ztopic::{operators::Interval, references::RawRef, storages::Broadcast, StorageManager, Topic, TopicManager};

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() {
    let manager = TopicManager::new(37);

    let mut my0 = manager.topic(MyTopic2::new("demo", 3)).with_key(0);
    let mut my1 = manager.topic(MyTopic2::new("demo", 3)).with_key(1);
    let mut my2 = manager.topic(MyTopic2::new("demo", 3)).with_key(2);

    tokio::spawn(async move {
        while let Some(msg) = my0.next().await {
            match msg {
                Ok(msg) => println!("     0: {:?}", msg),
                Err(error) => eprintln!("{:?}", error),
            }
        }
    });
    tokio::spawn(async move {
        while let Some(msg) = my1.next().await {
            match msg {
                Ok(msg) => println!("     1: {:?}", msg),
                Err(error) => eprintln!("{:?}", error),
            }
        }
    });
    tokio::spawn(async move {
        while let Some(msg) = my2.next().await {
            match msg {
                Ok(msg) => println!("     2: {:?}", msg),
                Err(error) => eprintln!("{:?}", error),
            }
        }
    });

    tokio::time::sleep(Duration::from_secs(60)).await;
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

    type Storage = Broadcast<Self::Output>;

    fn storage(&self) -> Self::Storage {
        Broadcast::new(4096)
    }

    fn mount(&self, manager: TopicManager<usize>, storage: StorageManager<(), Self::Output, Self::Storage>) -> BoxStream<'static, Result<(), Self::Error>> {
        manager
            .topic(Interval::new(Duration::from_millis(1000)))
            .into_stream()
            .map(move |_| {
                storage.insert(String::from("hello world"));
                Ok(())
            })
            .boxed()
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

impl Topic<usize, usize> for MyTopic2 {
    type Output = String;

    type Error = Error;

    type References = RawRef<String>;

    type Storage = Broadcast<Self::Output>;

    fn topic_id(&self) -> impl std::fmt::Debug + std::hash::Hash {
        self.args.clone()
    }

    fn storage(&self) -> Self::Storage {
        Broadcast::new(4096)
    }

    fn mount(&self, manager: TopicManager<usize>, storage: StorageManager<usize, Self::Output, Self::Storage>) -> BoxStream<'static, Result<(), Self::Error>> {
        manager
            .topic(MyTopic)
            .into_stream()
            .map(move |event| match event {
                Ok(event) => {
                    let key = rand::random::<usize>() % 3;
                    println!("key: {}, event: {:?}", key, event);
                    storage.insert_with(key, format!("{}, {}", *event, rand::random::<u8>()));
                    Ok(())
                }
                Err(error) => Err(error),
            })
            .boxed()
    }
}
