use std::{convert::Infallible, fmt::Debug};

use futures::{stream::BoxStream, StreamExt};

use crate::{Topic, TopicManager};

pub struct EmptyTopic<T, L, S> {
    label: L,
    marker: std::marker::PhantomData<(T, S)>,
}

impl<T, L, S> EmptyTopic<T, L, S> {
    pub fn new(label: L) -> Self {
        Self {
            label,
            marker: std::marker::PhantomData,
        }
    }
}

impl<T, S> Default for EmptyTopic<T, &'static str, S> {
    fn default() -> Self {
        Self::new("default")
    }
}

impl<T, L, S> Topic<S> for EmptyTopic<T, L, S>
where
    T: Send + Sync + 'static,
    S: Send + Sync + 'static,
    L: Debug,
{
    type Output = T;

    type Error = Infallible;

    fn topic(&self) -> String {
        format!("{:?}", self.label)
    }

    fn init(&self, _: &TopicManager<S>) -> BoxStream<'static, Result<Self::Output, Self::Error>> {
        futures::stream::empty().boxed()
    }
}
