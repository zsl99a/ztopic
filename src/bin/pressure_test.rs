use std::{convert::Infallible, time::Duration};

use futures::{stream::BoxStream, StreamExt};
use helium::{Interval, Topic, TopicManager};

#[tokio::main]
async fn main() {
    // let mut manager = TopicManager::new(());

    // let mut topic_1 = manager.topic(CpuPressureTestTopic::new("cpu_pressure_test_1".to_string()));

    // let mut topic_2 = manager.topic(CpuPressureTestTopic::new("cpu_pressure_test_2".to_string()));

    // tokio::spawn({
    //     let mut topic_1 = topic_1.clone();
    //     async move {
    //         while let Some(i) = topic_1.next().await {
    //             if rand::random::<u64>() % 100 == 0 {
    //                 println!("a = {:?}", i);
    //             }
    //         }
    //     }
    // });

    // tokio::spawn({
    //     let mut topic_2 = topic_2.clone();
    //     async move {
    //         while let Some(i) = topic_2.next().await {
    //             if rand::random::<u64>() % 100 == 0 {
    //                 println!("b = {:?}", i);
    //             }
    //         }
    //     }
    // });

    // tokio::spawn(async move {
    //     loop {
    //         tokio::select! {
    //             Some(i) = topic_1.next() => {
    //                 if rand::random::<u64>() % 100 == 0 {
    //                     println!("c = {:?}", i);
    //                 }
    //                 println!("ins = {:?}", topic_1.elapsed());
    //             }
    //             Some(i) = topic_2.next() => {
    //                 if rand::random::<u64>() % 100 == 0 {
    //                     println!("c = {:?}", i);
    //                 }
    //                 println!("ins = {:?}", topic_2.elapsed());
    //             }
    //         }
    //     }
    // });

    // tokio::spawn(async move {
    //     let mut topic_all = select_all(vec![topic_1, topic_2]);

    //     while let Some(i) = topic_all.next().await {
    //         if rand::random::<u64>() % 100 == 0 {
    //             println!("c = {:?}", i);
    //         }
    //     }
    // });

    tokio::time::sleep(tokio::time::Duration::from_secs(u64::MAX)).await;
}

pub struct CpuPressureTestTopic {
    name: String,
}

impl CpuPressureTestTopic {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl Topic<()> for CpuPressureTestTopic {
    type Output = f64;

    type Error = Infallible;

    fn topic(&self) -> String {
        self.name.clone()
    }

    fn init(&self, manager: &TopicManager<()>) -> BoxStream<'static, Result<Self::Output, Self::Error>> {
        let interval = manager.topic(Interval::new(Duration::from_micros(1)));
        interval
            .map(|_| {
                let mut x = 0.0;
                for _ in 0..30_0000 {
                    x += rand::random::<f64>() % 10.0;
                }
                Ok(x)
            })
            .boxed()
    }
}
