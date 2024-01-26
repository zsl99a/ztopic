use std::convert::Infallible;

use futures::{stream::BoxStream, StreamExt};

use crate::{Topic, TopicManager};

pub struct EmptyTopic<T, S> {
    marker: std::marker::PhantomData<(T, S)>,
}

impl<T, S> EmptyTopic<T, S> {
    pub fn new() -> Self {
        Self {
            marker: std::marker::PhantomData,
        }
    }
}

impl<T, S> Default for EmptyTopic<T, S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, S> Topic<S> for EmptyTopic<T, S>
where
    T: Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    type Output = T;

    type Error = Infallible;

    fn init(&self, _: &TopicManager<S>) -> BoxStream<'static, Result<Self::Output, Self::Error>> {
        futures::stream::empty().boxed()
    }
}
