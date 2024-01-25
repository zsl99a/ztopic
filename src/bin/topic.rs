use futures::{channel::mpsc, stream::BoxStream, SinkExt, StreamExt};
use helium::{Topic, TopicManager};

#[tokio::main]
async fn main() {
    let manager = TopicManager::new(());

    let mut topic = manager.topic(MyTopic::new("my".into()));

    while let Some(Ok(i)) = topic.next().await {
        println!("i = {}", i);

        if i == 300 {
            break;
        }
    }

    println!("{:?}", manager);

    drop(topic);

    println!("{:?}", manager);

    println!("Done");
}

pub struct MyTopic {
    name: String,
}

impl MyTopic {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl<S> Topic<S> for MyTopic {
    type Output = usize;

    type Error = ();

    fn topic(&self) -> String {
        self.name.clone()
    }

    fn init(&self, _manager: &TopicManager<S>) -> BoxStream<'static, Result<Self::Output, Self::Error>> {
        let (mut tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            for i in 0..300 {
                tx.send(i).await.unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
        });

        rx.scan(0, |f, _i| {
            *f += 1;
            futures::future::ready(Some(*f))
        })
        .map(Ok)
        .boxed()
    }
}
