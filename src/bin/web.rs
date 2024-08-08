use std::{
    convert::Infallible,
    net::SocketAddr,
    time::{Duration, Instant},
};

use anyhow::Result;
use futures::{
    stream::{select_all, BoxStream},
    StreamExt,
};
use helium::{Interval, Topic, TopicManager};

#[tokio::main]
async fn main() -> Result<()> {
    let port = dotenvy::var("PORT").unwrap_or_else(|_| "8080".into()).parse::<u16>()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let manager = TopicManager::new(());

    let mut topic = manager
        .topic(IntervalMulti::new(Duration::from_secs(1), "hello".into()))
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
        let mut idx = 0;
        while let Some(i) = topic.next().await {
            println!("a = {:?}", i);
            idx += 1;
            if idx == 10 {
                break;
            }
        }
    });

    let mut topic = manager
        .topic(IntervalMulti::new(Duration::from_secs(1), "world".into()))
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
        let mut idx = 0;
        while let Some(i) = topic.next().await {
            println!("a = {:?}", i);
            idx += 1;
            if idx == 5 {
                break;
            }
        }
    });

    let routes = helium::helium_routes(manager);

    let tcp_listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve::serve(tcp_listener, routes.into_make_service()).await?;

    Ok(())
}

pub struct IntervalMulti {
    dur: Duration,
    tag: String,
}

impl IntervalMulti {
    pub fn new(dur: Duration, tag: String) -> Self {
        Self { dur, tag }
    }
}

impl<S: 'static> Topic<S> for IntervalMulti
where
    S: Send + Sync + 'static,
{
    type Output = Instant;

    type Error = Infallible;

    fn topic(&self) -> String {
        format!("{:?} {}", self.dur, self.tag)
    }

    fn init(&self, manager: &TopicManager<S>) -> BoxStream<'static, Result<Self::Output, Self::Error>> {
        let interval1 = manager.topic(Interval::new(self.dur));
        let interval2 = manager.topic(Interval::new(self.dur));
        select_all(vec![interval1, interval2]).boxed()
    }
}
