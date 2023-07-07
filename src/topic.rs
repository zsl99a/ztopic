use futures::{stream::BoxStream, StreamExt};

pub trait Topic<I> {
    type Output;

    type Error;

    fn topic(&self) -> String;

    fn init(&self) -> BoxStream<Self::Output>;
}

pub struct MyTopic;

impl Topic<()> for MyTopic {
    type Output = ();

    type Error = ();

    fn topic(&self) -> String {
        format!("MyTopic")
    }

    fn init(&self) -> BoxStream<Self::Output> {
        let f1 = futures::stream::iter(0..10);
        let f2 = futures::stream::iter(11..20);
        f1.chain(f2).map(|_| ()).boxed()
    }
}
