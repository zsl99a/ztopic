use std::{cmp::Ordering, hash::Hash, marker::Send, sync::Arc};

use futures::{future::BoxFuture, stream::BoxStream, Future, StreamExt};
use tokio::{sync::Notify, task::JoinSet};

use crate::{storages::StorageManager, FlowGroup, Storage};

type NewGroupHook<K, V, S> = dyn Fn(FlowGroup<K, V, S>) -> BoxFuture<'static, ()> + Send + Sync;

pub struct Flow<K, V, S>
where
    K: Clone + Default + Hash + Eq + Ord,
    S: Storage<V>,
{
    max_load: usize,
    storage: StorageManager<K, V, S>,
    new_group: Arc<NewGroupHook<K, V, S>>,
    join_sets: Vec<JoinSet<()>>,
    notify: Arc<Notify>,
}

impl<K, V, S> Flow<K, V, S>
where
    K: Clone + Default + Hash + Eq + Ord + Send + 'static,
    V: Send + 'static,
    S: Storage<V> + 'static,
{
    pub fn new<F, Fut>(max_load: usize, storage: StorageManager<K, V, S>, new_group: F) -> Self
    where
        F: Fn(FlowGroup<K, V, S>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + Sync + 'static,
    {
        Self {
            max_load,
            storage,
            new_group: Arc::new(move |group| Box::pin(new_group(group))),
            join_sets: vec![],
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn to_stream(mut self) -> BoxStream<'static, ()> {
        async_stream::stream! {
            loop {
                self.refresh();
                yield;
                self.storage.notified().await;
            }
        }
        .boxed()
    }

    fn refresh(&mut self) {
        let keys = self.storage.registry().keys().cloned().collect::<Vec<_>>();
        let group_size = keys.len() / self.max_load + 1;
        loop {
            match self.join_sets.len().cmp(&group_size) {
                Ordering::Less => {
                    let mut join_set = JoinSet::new();
                    join_set.spawn((self.new_group)(FlowGroup::new(
                        self.join_sets.len(),
                        self.max_load,
                        self.notify.clone(),
                        self.storage.clone(),
                    )));
                    self.join_sets.push(join_set);
                }
                Ordering::Greater => {
                    self.join_sets.pop();
                }
                Ordering::Equal => break,
            }
        }
        self.notify.notify_waiters();
    }
}
