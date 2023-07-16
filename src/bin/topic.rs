use futures::StreamExt;
use helium::{MyTopic, TopicManager};

#[tokio::main]
async fn main() {
    let mut manager = TopicManager::new(());

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
