use std::convert::Infallible;

use futures::{stream::BoxStream, StreamExt};
use helium::{Topic, TopicManager};

#[tokio::main]
async fn main() {
    let manager = TopicManager::new(());
    let mut symbol_topic = manager.topic(PublicEventTopic::new());

    tokio::spawn(async move {
        tokio::select! {
            event = symbol_topic.next() => {
                println!("1, {event:?}");
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {}
        }
    });

    let mut symbol_topic = manager.topic(PublicEventTopic::new());
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        while let Some(event) = symbol_topic.next().await {
            println!("[BTC] event: {event:?}");
        }
    });

    tokio::time::sleep(std::time::Duration::from_secs(u64::MAX)).await;
}

pub struct PublicEventTopic {}

impl PublicEventTopic {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for PublicEventTopic {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Topic<S> for PublicEventTopic
where
    S: Send + Sync + 'static,
{
    type Output = PublicEvent;

    type Error = Infallible;

    fn topic(&self) -> String {
        "public".to_string()
    }

    fn init(&self, _: &TopicManager<S>) -> BoxStream<'static, Result<Self::Output, Self::Error>> {
        async_stream::stream! {
            let mut tick = false;
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let event = String::from(if tick {"BTC/USDT"} else {"ETH/USDT"});
                yield Ok(PublicEvent::Ticker(event.clone()));
                tick = !tick;

                println!("insert event: {event}");
            }
        }
        .boxed()
    }
}

#[derive(Debug, Clone)]
pub enum PublicEvent {
    Ticker(String),
    Trade(String),
}
